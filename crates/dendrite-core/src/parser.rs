use super::line_map::LineMap;
use crate::model::{Block, Heading, LinkKind, Point, TextRange, WikiLinkFormat};
use pulldown_cmark::{Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TagEnd};

pub(crate) struct DocLink {
    pub target: String,
    pub raw_target: String,
    pub alias: Option<String>,
    pub anchor: Option<String>,
    pub range: TextRange,
    pub kind: LinkKind,
}

pub(crate) struct ParseResult {
    pub links: Vec<DocLink>,
    pub headings: Vec<Heading>,
    pub blocks: Vec<Block>,
    pub title: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
    pub content_start_offset: usize,
    pub digest: String,
}

/// Computes a hex-encoded SHA256 digest of the given text
pub(crate) fn compute_digest(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text);
    format!("{:x}", hasher.finalize())
}

/// Parse markdown content into structured data
pub(crate) fn parse_markdown(text: &str, supported_kinds: &[LinkKind]) -> ParseResult {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    // Dynamic configuration based on supported kinds
    let mut enable_wikilinks = false;
    let mut wikilink_format = WikiLinkFormat::AliasFirst; // Default

    for kind in supported_kinds {
        match kind {
            LinkKind::WikiLink(format) | LinkKind::EmbeddedWikiLink(format) => {
                enable_wikilinks = true;
                wikilink_format = *format;
            }
            _ => {}
        }
    }

    if enable_wikilinks {
        options.insert(Options::ENABLE_WIKILINKS);
    }
    // Pulldown doesn't have a separate flag for autolinks in new versions,
    // it's often enabled by default or via linkify. But check options if needed.
    // For now we just use standard parser behavior for autolinks if they appear.

    let parser = Parser::new_ext(text, options);
    let line_map = LineMap::new(text);

    let mut links = Vec::new();
    let mut headings = Vec::new();
    let mut blocks = Vec::new();
    let mut title = None;
    let mut frontmatter = None;
    let mut content_start_offset = 0;

    let mut in_heading = false;
    let mut current_heading_level = 0;
    let mut pending_heading_text: Option<(String, Point)> = None;
    let mut in_frontmatter = false;
    let mut frontmatter_content = String::new();

    struct PendingLink {
        raw_left: String,
        start_point: Point,
        is_embedded: bool,
        is_wikilink: bool,
        collector: String,
    }
    let mut pending_link: Option<PendingLink> = None;

    let mut current_block_text = String::new();
    let mut current_block_start: Option<Point> = None;

    let mut in_block_container = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Paragraph) | Event::Start(Tag::Item) => {
                in_block_container = true;
                current_block_start = Some(line_map.offset_to_point(text, range.start));
                current_block_text.clear();
            }
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) => {
                if in_block_container {
                    if let Some(pos) = current_block_text.rfind(" ^") {
                        let id = current_block_text[pos + 2..].trim();
                        if !id.is_empty() && id.chars().all(|c| c.is_alphanumeric() || c == '-') {
                            blocks.push(Block {
                                id: id.to_string(),
                                range: TextRange {
                                    start: current_block_start.unwrap(),
                                    end: line_map.offset_to_point(text, range.end),
                                },
                            });
                        }
                    }
                }
                in_block_container = false;
            }

            Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                in_frontmatter = true;
            }
            Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                in_frontmatter = false;
                if let Ok(json) = serde_yaml::from_str::<serde_json::Value>(&frontmatter_content) {
                    if let Some(t) = json.get("title").and_then(|v| v.as_str()) {
                        title = Some(t.to_string());
                    }
                    frontmatter = Some(json);
                }
                content_start_offset = range.end;
            }

            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_heading_level = level as u8;
                let start = line_map.offset_to_point(text, range.start);
                pending_heading_text = Some((String::new(), start));
            }
            Event::End(TagEnd::Heading(..)) => {
                if let Some((heading_text, start_point)) = pending_heading_text.take() {
                    let end_point = line_map.offset_to_point(text, range.end);
                    let trimmed_text = heading_text.trim().to_string();

                    if !trimmed_text.is_empty() {
                        headings.push(Heading {
                            level: current_heading_level,
                            text: trimmed_text.clone(),
                            range: TextRange {
                                start: start_point,
                                end: end_point,
                            },
                        });

                        if current_heading_level == 1 && title.is_none() {
                            title = Some(trimmed_text);
                        }
                    }
                }
                in_heading = false;
            }

            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                if matches!(link_type, LinkType::Autolink | LinkType::Email) {
                    let start = line_map.offset_to_point(text, range.start);
                    let end = line_map.offset_to_point(text, range.end);
                    links.push(DocLink {
                        target: dest_url.to_string(),
                        raw_target: dest_url.to_string(),
                        alias: None,
                        anchor: None,
                        range: TextRange { start, end },
                        kind: LinkKind::AutoLink,
                    });
                } else {
                    let is_wikilink = matches!(link_type, LinkType::WikiLink { .. });
                    if is_wikilink && !enable_wikilinks {
                        continue;
                    }

                    let start = line_map.offset_to_point(text, range.start);

                    pending_link = Some(PendingLink {
                        raw_left: dest_url.to_string(),
                        start_point: start,
                        is_embedded: false,
                        is_wikilink,
                        collector: String::new(),
                    });
                }
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                ..
            }) => {
                let is_wikilink = matches!(link_type, LinkType::WikiLink { .. });
                if is_wikilink && !enable_wikilinks {
                    continue;
                }

                let start = line_map.offset_to_point(text, range.start);
                pending_link = Some(PendingLink {
                    raw_left: dest_url.to_string(),
                    start_point: start,
                    is_embedded: true,
                    is_wikilink,
                    collector: String::new(),
                });
            }
            Event::End(TagEnd::Link { .. }) | Event::End(TagEnd::Image) => {
                if let Some(pending) = pending_link.take() {
                    let mut end_offset = range.end;
                    // For wikilinks, pulldown_cmark might report range ending before the last ']'
                    if pending.is_wikilink {
                        while end_offset < text.len() && text.as_bytes()[end_offset] == b']' {
                            end_offset += 1;
                        }
                    }

                    let end_point = line_map.offset_to_point(text, end_offset);
                    let left = pending.raw_left.trim();
                    let right = pending.collector.trim();

                    let (mut final_target, alias, kind) = if pending.is_wikilink {
                        let (target, alias) = match wikilink_format {
                            WikiLinkFormat::AliasFirst => {
                                if left == right || right.is_empty() {
                                    (left.to_string(), None)
                                } else {
                                    (right.to_string(), Some(left.to_string()))
                                }
                            }
                            WikiLinkFormat::TargetFirst => {
                                if left == right || right.is_empty() {
                                    (left.to_string(), None)
                                } else {
                                    (left.to_string(), Some(right.to_string()))
                                }
                            }
                        };
                        let kind = if pending.is_embedded {
                            LinkKind::EmbeddedWikiLink(wikilink_format)
                        } else {
                            LinkKind::WikiLink(wikilink_format)
                        };
                        (target, alias, kind)
                    } else {
                        // Standard Markdown link or Image: [alias](target) or ![alt](target)
                        let kind = if pending.is_embedded {
                            LinkKind::MarkdownImage
                        } else {
                            LinkKind::MarkdownLink
                        };

                        (
                            left.to_string(),
                            if right.is_empty() {
                                None
                            } else {
                                Some(right.to_string())
                            },
                            kind,
                        )
                    };

                    // Final Filter: Check if this resolved Kind is in our supported list
                    // (For equality check with payload, we can use contains if payloads are equal)
                    if supported_kinds.contains(&kind) {
                        let mut anchor = None;
                        if let Some(pos) = final_target.find('#') {
                            anchor = Some(final_target[pos + 1..].to_string());
                            final_target.truncate(pos);
                        }

                        links.push(DocLink {
                            target: final_target,
                            raw_target: left.to_string(), // Preserve the original text
                            alias,
                            anchor,
                            range: TextRange {
                                start: pending.start_point,
                                end: end_point,
                            },
                            kind,
                        });
                    }
                }
            }

            Event::Text(cow_str) => {
                let text = cow_str.as_ref();
                // BlockRef removed for now as unconfigured

                if in_frontmatter {
                    frontmatter_content.push_str(text);
                } else if let Some(pending) = pending_link.as_mut() {
                    pending.collector.push_str(text);
                } else if in_heading {
                    if let Some((ref mut heading_text, _)) = pending_heading_text.as_mut() {
                        if !heading_text.is_empty() {
                            heading_text.push(' ');
                        }
                        heading_text.push_str(text);
                    }
                }

                if in_block_container {
                    current_block_text.push_str(text);
                }
            }
            // AutoLink handling
            _ => {}
        }
    }

    let digest = compute_digest(text);

    ParseResult {
        links,
        headings,
        blocks,
        title,
        frontmatter,
        content_start_offset,
        digest,
    }
}

/// Finds the line containing "updated: ..." within the first `limit` bytes of `text`.
/// Returns the range of the value part.
pub fn get_updated_field_range(text: &str, limit: usize) -> Option<TextRange> {
    let limit = std::cmp::min(limit, text.len());
    let search_area = &text[..limit];
    let line_map = LineMap::new(text);

    for line in search_area.lines() {
        if let Some(pos) = line.find("updated:") {
            let value_start = pos + "updated:".len();
            let value_part = &line[value_start..];
            let trimmed_val = value_part.trim();

            if let Some(val_pos_in_line) = line.find(trimmed_val) {
                let current_line_start = search_area.find(line).unwrap();
                let start_offset = current_line_start + val_pos_in_line;
                let end_offset = start_offset + trimmed_val.len();

                return Some(TextRange {
                    start: line_map.offset_to_point(text, start_offset),
                    end: line_map.offset_to_point(text, end_offset),
                });
            } else {
                // If it's empty like "updated: ", just return the end of "updated: "
                let current_line_start = search_area.find(line).unwrap();
                let start_offset = current_line_start + value_start;
                let end_offset = start_offset + value_part.len();
                return Some(TextRange {
                    start: line_map.offset_to_point(text, start_offset),
                    end: line_map.offset_to_point(text, end_offset),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WikiLinkFormat;

    fn default_kinds() -> Vec<LinkKind> {
        vec![
            LinkKind::WikiLink(WikiLinkFormat::AliasFirst),
            LinkKind::EmbeddedWikiLink(WikiLinkFormat::AliasFirst),
            LinkKind::MarkdownLink,
            // AutoLink not enabled by default for these tests unless specified
        ]
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\ntitle: My Note\nid: 123\n---\n# Content";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.title, Some("My Note".to_string()));
        assert!(result.frontmatter.is_some());
        let fm = result.frontmatter.unwrap();
        assert_eq!(fm["title"], "My Note");
        assert_eq!(fm["id"], 123);
    }

    #[test]
    fn test_parse_digest() {
        let content1 = "Content A";
        let content2 = "Content A";
        let content3 = "Content B";

        let result1 = parse_markdown(content1, &default_kinds());
        let result2 = parse_markdown(content2, &default_kinds());
        let result3 = parse_markdown(content3, &default_kinds());

        assert_eq!(result1.digest, result2.digest);
        assert_ne!(result1.digest, result3.digest);
        assert_eq!(result1.digest.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_parse_wiki_link() {
        let content = "# Note 1\n\n[[note2]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1, "Should parse one wiki link");
        assert_eq!(
            result.links[0].target, "note2",
            "Link target should be 'note2'"
        );
        assert!(matches!(result.links[0].kind, LinkKind::WikiLink { .. }));
    }

    #[test]
    fn test_parse_multiple_links() {
        let content = "# Note 1\n\n[[note2]] and [[note3]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 2, "Should parse two wiki links");
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[1].target, "note3");
    }

    #[test]
    fn test_parse_wiki_link_with_alias() {
        let content = "[[My Alias | note2]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn test_parse_wiki_link_with_anchor() {
        let content = "[[note2#section-1]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].anchor, Some("section-1".to_string()));
        assert_eq!(result.links[0].alias, None);
    }

    #[test]
    fn test_parse_embedded_wiki_link() {
        let content = "![[image.png]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "image.png");
        assert!(matches!(
            result.links[0].kind,
            LinkKind::EmbeddedWikiLink { .. }
        ));
    }

    #[test]
    fn test_parse_embedded_wiki_link_with_alias_and_anchor() {
        let content = "![[alias | a.link.to.note#with-anchor]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "a.link.to.note");
        assert_eq!(result.links[0].anchor, Some("with-anchor".to_string()));
        assert_eq!(result.links[0].alias, Some("alias".to_string()));
        assert!(matches!(
            result.links[0].kind,
            LinkKind::EmbeddedWikiLink { .. }
        ));
    }

    #[test]
    fn test_parse_wiki_link_with_spaces_and_range() {
        let content = "[[alias | target]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "target");
        assert_eq!(result.links[0].alias, Some("alias".to_string()));
        assert_eq!(result.links[0].range.start.col, 0);
        assert_eq!(result.links[0].range.end.col, 18);
    }

    #[test]
    fn test_parse_embedded_wiki_link_range() {
        let content = "![[image.png]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].range.start.col, 0);
        assert_eq!(result.links[0].range.end.col, 14);
    }

    #[test]
    fn test_parse_wiki_link_with_alias_and_anchor() {
        let content = "[[My Alias|note2#section-1]]";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].anchor, Some("section-1".to_string()));
        assert_eq!(result.links[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn test_parse_headings() {
        let content = "# Title\n\n## Section";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.headings[1].text, "Section");
        assert_eq!(result.title, Some("Title".to_string()));
    }

    #[test]
    fn test_issue_6_alias_and_trailing_text() {
        let content = "  - 和[[Alias|a.b.c]]类似。";
        let result = parse_markdown(content, &default_kinds());
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.alias, Some("Alias".to_string()));
        assert_eq!(link.target, "a.b.c");
        assert_eq!(link.range.start.col, 5);
        assert_eq!(link.range.end.col, 20);
    }

    #[test]
    fn test_issue_5_embedded_image_link() {
        let content = "同时参考，![[a.b.c.d.e]]";
        let result = parse_markdown(content, &default_kinds());
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert!(matches!(link.kind, LinkKind::EmbeddedWikiLink { .. }));
        assert_eq!(link.target, "a.b.c.d.e");
        assert_eq!(link.range.start.col, 5);
        assert_eq!(link.range.end.col, 19);
    }

    #[test]
    fn test_parse_wiki_link_with_anchor_range() {
        let content = "Check [[target#section-1]] highlight";
        let result = parse_markdown(content, &default_kinds());
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.target, "target");
        assert_eq!(link.anchor, Some("section-1".to_string()));
        assert_eq!(link.range.start.col, 6);
        assert_eq!(link.range.end.col, 26);
    }

    #[test]
    fn test_parse_wiki_link_with_block_id_range() {
        let content = "See [[target#^block-id]] for details";
        let result = parse_markdown(content, &default_kinds());
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.target, "target");
        assert_eq!(link.anchor, Some("^block-id".to_string()));
        assert_eq!(link.range.start.col, 4);
        assert_eq!(link.range.end.col, 24);
    }

    #[test]
    fn test_content_offset_calculation() {
        let content_with_fm = "---\ntitle: Hello\n---\nActual content starts here.";
        let result_fm = parse_markdown(content_with_fm, &default_kinds());
        assert_eq!(result_fm.content_start_offset, 20);

        let content_no_fm = "No frontmatter here.";
        let result_no_fm = parse_markdown(content_no_fm, &default_kinds());
        assert_eq!(result_no_fm.content_start_offset, 0);
    }

    #[test]
    fn test_parse_markdown_link() {
        let content = "Check [My Alias](note2.md) link";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.target, "note2.md");
        assert_eq!(link.alias, Some("My Alias".to_string()));
        assert!(matches!(link.kind, LinkKind::MarkdownLink));

        assert_eq!(link.range.start.col, 6);
        assert_eq!(link.range.end.col, 26);
    }

    #[test]
    fn test_parse_markdown_link_with_anchor() {
        let content = "[Target](note.md#section-1)";
        let result = parse_markdown(content, &default_kinds());

        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.target, "note.md");
        assert_eq!(link.anchor, Some("section-1".to_string()));
    }

    #[test]
    fn test_parse_markdown_image() {
        let content = "![Alt Text](image.png)";
        // To test image, we need to enable MarkdownImage or allow fallback.
        // Current logic maps standard image to MarkdownImage if enabled, or MarkdownLink if not?
        // Let's enable MarkdownImage in custom config.
        let mut kinds = default_kinds();
        kinds.push(LinkKind::MarkdownImage);

        let result = parse_markdown(content, &kinds);

        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.target, "image.png");
        assert!(matches!(link.kind, LinkKind::MarkdownImage));
        assert_eq!(link.alias, Some("Alt Text".to_string()));
    }
}
