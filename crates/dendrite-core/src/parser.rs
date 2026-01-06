use super::line_map::LineMap;
use crate::model::{Block, Heading, LinkKind, Point, TextRange};
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
    pub digest: String,
}

pub(crate) fn parse_markdown(text: &str) -> ParseResult {
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
                current_block_start = Some(line_map.offset_to_point(range.start));
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
                                    end: line_map.offset_to_point(range.end),
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
            }

            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_heading_level = level as u8;
                let start = line_map.offset_to_point(range.start);
                pending_heading_text = Some((String::new(), start));
            }
            Event::End(TagEnd::Heading(..)) => {
                if let Some((heading_text, start_point)) = pending_heading_text.take() {
                    let end_point = line_map.offset_to_point(range.end);
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
                    let start = line_map.offset_to_point(range.start);
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
                    let start = line_map.offset_to_point(range.start);
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
                    let end_point = line_map.offset_to_point(range.end);

                    let left = raw_left.trim();
                    let right = raw_right.trim();

                    // Logic update: User requested [[Alias | Target]] format.
                    // Standard parser yields Left=Dest (Part 1), Right=Text (Part 2).
                    // So Left="Alias", Right="Target".

                    let (mut final_target, alias) = if left == right || right.is_empty() {
                        // Case [[Target]] (where left==right) or [[Target|]] (where right is empty?? no, then right is "")
                        // Actually if [[Target]], left="Target", right="Target".
                        (left.to_string(), None)
                    } else {
                        // Case [[Alias | Target]]
                        (right.to_string(), Some(left.to_string()))
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
        digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\ntitle: My Note\nid: 123\n---\n# Content";
        let result = parse_markdown(content);

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

        let result1 = parse_markdown(content1);
        let result2 = parse_markdown(content2);
        let result3 = parse_markdown(content3);

        assert_eq!(result1.digest, result2.digest);
        assert_ne!(result1.digest, result3.digest);
        assert_eq!(result1.digest.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_parse_wiki_link() {
        let content = "# Note 1\n\n[[note2]]";
        let result = parse_markdown(content);

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
        let result = parse_markdown(content);

        assert_eq!(result.links.len(), 2, "Should parse two wiki links");
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[1].target, "note3");
    }

    #[test]
    fn test_parse_wiki_link_with_alias() {
        let content = "[[My Alias | note2]]";
        let result = parse_markdown(content);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn test_parse_wiki_link_with_anchor() {
        let content = "[[note2#section-1]]";
        let result = parse_markdown(content);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].anchor, Some("section-1".to_string()));
        assert_eq!(result.links[0].alias, None);
    }

    #[test]
    fn test_parse_embedded_wiki_link() {
        let content = "![[image.png]]";
        let result = parse_markdown(content);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "image.png");
        assert_eq!(result.links[0].kind, LinkKind::EmbeddedWikiLink);
    }

    #[test]
    fn test_parse_embedded_wiki_link_with_alias_and_anchor() {
        let content = "![[alias | a.link.to.note#with-anchor]]";
        let result = parse_markdown(content);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "a.link.to.note");
        assert_eq!(result.links[0].anchor, Some("with-anchor".to_string()));
        assert_eq!(result.links[0].alias, Some("alias".to_string()));
        assert_eq!(result.links[0].kind, LinkKind::EmbeddedWikiLink);
    }

    #[test]
    fn test_parse_wiki_link_with_alias_and_anchor() {
        let content = "[[My Alias|note2#section-1]]";
        let result = parse_markdown(content);

        assert_eq!(result.links.len(), 1);
        assert_eq!(result.links[0].target, "note2");
        assert_eq!(result.links[0].anchor, Some("section-1".to_string()));
        assert_eq!(result.links[0].alias, Some("My Alias".to_string()));
    }

    #[test]
    fn test_parse_headings() {
        let content = "# Title\n\n## Section";
        let result = parse_markdown(content);

        assert_eq!(result.headings[1].text, "Section");
        assert_eq!(result.title, Some("Title".to_string()));
    }

    #[test]
    fn test_parse_explicit_blocks() {
        let content = "Paragraph with id ^my-id\n\n- List item ^list-id\n- Normal item";
        let result = parse_markdown(content);

        assert_eq!(result.blocks.len(), 2);
        assert_eq!(result.blocks[0].id, "my-id");
        assert_eq!(result.blocks[1].id, "list-id");

        // Check range (line 0 for paragraph, line 2 for list item)
        assert_eq!(result.blocks[0].range.start.line, 0);
        assert_eq!(result.blocks[1].range.start.line, 2);
    }
}
