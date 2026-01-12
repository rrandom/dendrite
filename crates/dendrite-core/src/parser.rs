use super::line_map::LineMap;
use crate::model::{Block, Heading, LinkKind, Point, TextRange};
use crate::syntax::WikiLinkFormat;
use pulldown_cmark::{Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TagEnd};

pub(crate) struct DocLink {
    pub target: String,
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

pub(crate) fn parse_markdown(text: &str, wikilink_format: WikiLinkFormat) -> ParseResult {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_WIKILINKS);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

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
    let mut pending_wiki_link: Option<(String, Option<String>, String, Point, bool, String)> = None;

    let mut in_frontmatter = false;
    let mut frontmatter_content = String::new();

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
                let is_wikilink = matches!(link_type, LinkType::WikiLink { .. });
                if is_wikilink {
                    let start = line_map.offset_to_point(text, range.start);
                    let full_dest = dest_url.to_string();
                    // Delay anchor parsing because the target might come from the right side (alias position)
                    // We store (raw_left, parsed_anchor_placeholder, raw_left_clone, start, is_embedded, collector)
                    pending_wiki_link = Some((
                        full_dest.clone(),
                        None,
                        full_dest,
                        start,
                        false,
                        String::new(),
                    ));
                }
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                ..
            }) => {
                let is_wikilink = matches!(link_type, LinkType::WikiLink { .. });
                if is_wikilink {
                    let start = line_map.offset_to_point(text, range.start);
                    let full_dest = dest_url.to_string();
                    pending_wiki_link = Some((
                        full_dest.clone(),
                        None,
                        full_dest,
                        start,
                        true, // is_embedded
                        String::new(),
                    ));
                }
            }
            Event::End(TagEnd::Link { .. }) | Event::End(TagEnd::Image) => {
                if let Some((raw_left, _, _, start_point, is_embedded, raw_right)) =
                    pending_wiki_link.take()
                {
                    // Fix: pulldown_cmark might report range ending before the last ']'
                    // Check if we need to extend the range
                    // Note: range.end is an offset
                    let mut end_offset = range.end;
                    while end_offset < text.len() && text.as_bytes()[end_offset] == b']' {
                        end_offset += 1;
                    }

                    let end_point = line_map.offset_to_point(text, end_offset);

                    let left = raw_left.trim();
                    let right = raw_right.trim();

                    // Use wikilink_format to determine parsing order
                    let (mut final_target, alias) = match wikilink_format {
                        WikiLinkFormat::AliasFirst => {
                            // Dendron: [[alias|target]]
                            if left == right || right.is_empty() {
                                (left.to_string(), None)
                            } else {
                                (right.to_string(), Some(left.to_string()))
                            }
                        }
                        WikiLinkFormat::TargetFirst => {
                            // Obsidian: [[target|alias]]
                            if left == right || right.is_empty() {
                                (left.to_string(), None)
                            } else {
                                (left.to_string(), Some(right.to_string()))
                            }
                        }
                    };

                    let mut anchor = None;
                    if let Some(pos) = final_target.find('#') {
                        anchor = Some(final_target[pos + 1..].to_string());
                        final_target.truncate(pos);
                    }

                    links.push(DocLink {
                        target: final_target,
                        alias,
                        anchor,
                        range: TextRange {
                            start: start_point,
                            end: end_point,
                        },
                        kind: if is_embedded {
                            LinkKind::EmbeddedWikiLink
                        } else {
                            LinkKind::WikiLink
                        },
                    });
                }
            }

            Event::Text(cow_str) => {
                let text = cow_str.as_ref();

                if in_frontmatter {
                    frontmatter_content.push_str(text);
                } else if let Some((_, _, _, _, _, ref mut collector)) = pending_wiki_link.as_mut()
                {
                    collector.push_str(text);
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
            _ => {}
        }
    }

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text);
    let digest = format!("{:x}", hasher.finalize());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\ntitle: My Note\nid: 123\n---\n# Content";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

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

        let result1 = parse_markdown(content1, WikiLinkFormat::AliasFirst);
        let result2 = parse_markdown(content2, WikiLinkFormat::AliasFirst);
        let result3 = parse_markdown(content3, WikiLinkFormat::AliasFirst);

        assert_eq!(result1.digest, result2.digest);
        assert_ne!(result1.digest, result3.digest);
        assert_eq!(result1.digest.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_parse_wiki_link() {
        let content = "# Note 1\n\n[[note2]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1, "Should parse one wiki link");
        assert_eq!(
            result.links[0].target, "note2",
            "Link target should be 'note2'"
        );
        assert_eq!(
            result.links[0].kind,
            LinkKind::WikiLink,
            "Link should be WikiLink"
        );
    }

    #[test]
    fn test_parse_multiple_links() {
        let content = "# Note 1\n\n[[note2]] and [[note3]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 2, "Should parse two wiki links");
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[1].target, "note3");
    }

    #[test]
    fn test_parse_wiki_link_with_alias() {
        let content = "[[My Alias | note2]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn test_parse_wiki_link_with_anchor() {
        let content = "[[note2#section-1]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].anchor, Some("section-1".to_string()));
        assert_eq!(result.links[0].alias, None);
    }

    #[test]
    fn test_parse_embedded_wiki_link() {
        let content = "![[image.png]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "image.png");
        assert_eq!(result.links[0].kind, LinkKind::EmbeddedWikiLink);
    }

    #[test]
    fn test_parse_embedded_wiki_link_with_alias_and_anchor() {
        let content = "![[alias | a.link.to.note#with-anchor]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "a.link.to.note");
        assert_eq!(result.links[0].anchor, Some("with-anchor".to_string()));
        assert_eq!(result.links[0].alias, Some("alias".to_string()));
        assert_eq!(result.links[0].kind, LinkKind::EmbeddedWikiLink);
    }

    #[test]
    fn test_parse_wiki_link_with_spaces_and_range() {
        let content = "[[alias | target]]";
        //             012345678901234567
        //             Start: 0, End: 18 (inclusive? range is usually half-open or end points to next char)
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "target");
        assert_eq!(result.links[0].alias, Some("alias".to_string()));

        // Check range
        assert_eq!(result.links[0].range.start.col, 0);
        // "[[alias | target]]" length is 18 chars.
        assert_eq!(result.links[0].range.end.col, 18);
    }

    #[test]
    fn test_parse_embedded_wiki_link_range() {
        let content = "![[image.png]]";
        //             01234567890123
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].range.start.col, 0);
        assert_eq!(result.links[0].range.end.col, 14);
    }

    #[test]
    fn test_parse_wiki_link_with_alias_and_anchor() {
        let content = "[[My Alias|note2#section-1]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].anchor, Some("section-1".to_string()));
        assert_eq!(result.links[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn test_parse_headings() {
        let content = "# Title\n\n## Section";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);

        assert_eq!(result.headings[1].text, "Section");
        assert_eq!(result.title, Some("Title".to_string()));
    }

    #[test]
    fn test_issue_6_alias_and_trailing_text() {
        let content = "  - 和[[Alias|a.b.c]]类似。";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.alias, Some("Alias".to_string()));
        assert_eq!(link.target, "a.b.c");

        // Verify range (UTF-16 code units)
        // "  - 和" -> 5 characters.
        // Link starts at character 5.
        // "[[Alias|a.b.c]]" -> 2 + 5 + 1 + 5 + 2 = 15 characters.
        // End character should be 5 + 15 = 20.
        assert_eq!(link.range.start.col, 5);
        assert_eq!(link.range.end.col, 20);
    }

    #[test]
    fn test_issue_5_embedded_image_link() {
        let content = "同时参考，![[a.b.c.d.e]]";
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.kind, LinkKind::EmbeddedWikiLink);
        assert_eq!(link.target, "a.b.c.d.e");

        // Start character: 5.
        // "![[a.b.c.d.e]]" -> 1 + 2 + 9 + 2 = 14 characters.
        // End character should be 5 + 14 = 19.
        assert_eq!(link.range.start.col, 5);
        assert_eq!(link.range.end.col, 19);
    }

    #[test]
    fn test_parse_wiki_link_with_anchor_range() {
        let content = "Check [[target#section-1]] highlight";
        //             01234567890123456789012345
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);
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
        //             0123456789012345678901234
        let result = parse_markdown(content, WikiLinkFormat::AliasFirst);
        assert_eq!(result.links.len(), 1);
        let link = &result.links[0];
        assert_eq!(link.target, "target");
        assert_eq!(link.anchor, Some("^block-id".to_string()));

        assert_eq!(link.range.start.col, 4);
        assert_eq!(link.range.end.col, 24);
    }

    #[test]
    fn test_content_offset_calculation() {
        // Case 1: With frontmatter
        let content_with_fm = "---\ntitle: Hello\n---\nActual content starts here.";
        let result_fm = parse_markdown(content_with_fm, WikiLinkFormat::AliasFirst);
        // "---\ntitle: Hello\n---" -> 3 + 1 + 12 + 1 + 3 = 20 chars
        // The offset should be exactly at the end of the block
        assert_eq!(result_fm.content_start_offset, 20);

        // Case 2: No frontmatter
        let content_no_fm = "No frontmatter here.";
        let result_no_fm = parse_markdown(content_no_fm, WikiLinkFormat::AliasFirst);
        assert_eq!(result_no_fm.content_start_offset, 0);
    }
}