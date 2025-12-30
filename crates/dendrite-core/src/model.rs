use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use uuid;

/// Core identity identifier
/// private
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct NoteId(pub uuid::Uuid);

pub type NoteKey = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolverId(pub &'static str);

/// Core internal coordinate system (0-based)
/// Does not directly use LSP Position to avoid coupling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextRange {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
    pub links: Vec<Link>,
    pub headings: Vec<Heading>,
}
/// Link entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub target: NoteId,
    pub range: TextRange,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkKind {
    WikiLink,     // [[target]]
    MarkdownLink, // [label](target)
}
