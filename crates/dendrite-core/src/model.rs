use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use uuid::Uuid;

/// Core identifier of a noate
/// Private and internal in the core library
/// The ONLY stable identifier across file change
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct NoteId(pub Uuid);

impl NoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NoteId {
    fn default() -> Self {
        Self(Uuid::nil())
    }
}

// Changeable name of a note
pub type NoteKey = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolverId(pub &'static str);

/// Core internal coordinate system (0-based)
/// Does not directly use LSP Position to avoid coupling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Point {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TextRange {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    pub id: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Note {
    #[allow(private_interfaces)]
    pub id: NoteId,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
    pub content_offset: usize,
    pub links: Vec<Link>,
    pub headings: Vec<Heading>,
    pub blocks: Vec<Block>,
    pub digest: Option<String>,
}
/// Link entity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Link {
    #[allow(private_interfaces)]
    pub target: NoteId,
    pub raw_target: String,
    pub alias: Option<String>,
    pub anchor: Option<String>,
    pub range: TextRange,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum LinkKind {
    #[default]
    WikiLink,         // [[target]]
    EmbeddedWikiLink, // ![[target]]
    MarkdownLink,     // [label](target)
}

/// Reference to a note for tree view
/// Used for serialization in LSP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteRef {
    /// Note ID as string (UUID)
    pub id: String,
    /// Note key (e.g., "foo.bar")
    pub key: Option<String>,
    /// File path as URI string
    pub path: Option<String>,
    /// Note title
    pub title: Option<String>,
}

/// Tree view structure for hierarchy display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeView {
    pub note: NoteRef,
    pub children: Vec<TreeView>,
}
