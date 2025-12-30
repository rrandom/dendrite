use super::line_map::LineMap;
use crate::model::{Heading, LinkKind, Point, TextRange};
use pulldown_cmark::{Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TagEnd};

pub(crate) struct DocLink {
    pub target: String,
    pub range: TextRange,
    pub kind: LinkKind,
}

pub(crate) struct ParseResult {
    pub links: Vec<DocLink>,
    pub headings: Vec<Heading>,
    pub title: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
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
    let mut title = None;
    let mut frontmatter = None;

    let mut in_heading = false;
    let mut current_heading_level = 0;
    let mut pending_heading_text: Option<(String, Point)> = None;
    let mut pending_wiki_link: Option<(String, Point, bool)> = None;

    let mut in_frontmatter = false;
    let mut frontmatter_content = String::new();

    for (event, range) in parser.into_offset_iter() {
        match event {
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
                    pending_wiki_link = Some((dest_url.to_string(), start, true));
                }
            }
            Event::End(TagEnd::Link { .. }) => {
                if let Some((target, start_point, is_wikilink)) = pending_wiki_link.take() {
                    if is_wikilink {
                        let end_point = line_map.offset_to_point(range.end);

                        links.push(DocLink {
                            target,
                            range: TextRange {
                                start: start_point,
                                end: end_point,
                            },
                            kind: LinkKind::WikiLink,
                        });
                    }
                }
            }

            Event::Text(cow_str) => {
                let text = cow_str.as_ref();

                if in_frontmatter {
                    frontmatter_content.push_str(text);
                } else if in_heading && pending_wiki_link.is_none() {
                    if let Some((ref mut heading_text, _)) = pending_heading_text.as_mut() {
                        if !heading_text.is_empty() {
                            heading_text.push(' ');
                        }
                        heading_text.push_str(text);
                    }
                }
            }
            _ => {}
        }
    }

    ParseResult {
        links,
        headings,
        title,
        frontmatter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wiki_link() {
        let content = "# Note 1\n\n[[note2]]";
        let result = parse_markdown(content);
        
        assert_eq!(result.links.len(), 1, "Should parse one wiki link");
        assert_eq!(result.links[0].target, "note2", "Link target should be 'note2'");
        assert_eq!(result.links[0].kind, LinkKind::WikiLink, "Link should be WikiLink");
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
    fn test_parse_headings() {
        let content = "# Title\n\n## Section";
        let result = parse_markdown(content);
        
        assert_eq!(result.headings.len(), 2);
        assert_eq!(result.headings[0].level, 1);
        assert_eq!(result.headings[0].text, "Title");
        assert_eq!(result.headings[1].level, 2);
        assert_eq!(result.headings[1].text, "Section");
        assert_eq!(result.title, Some("Title".to_string()));
    }
}