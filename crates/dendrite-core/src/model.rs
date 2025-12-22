use std::path::PathBuf;
use serde_json;

/// Core identity identifier (e.g., "foo.bar")
/// Always normalized to Unix style in memory (using / separator, no file extension)
pub type NoteId = String;

pub type LinkId = String;

/// Core internal coordinate system (0-based)
/// Does not directly use LSP Position to avoid coupling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct TextRange {
    pub start: Point,
    pub end: Point,
}

/// Note entity
#[derive(Debug, Clone)]
pub struct Note {
    /// Stable ID (e.g. "foo.bar")
    pub id: NoteId,

    /// Corresponding filesystem path
    /// None if this is a "Ghost Node" (virtual node)
    pub path: Option<PathBuf>,

    /// Note title (extracted from Frontmatter title or first h1)
    pub title: Option<String>,

    /// Raw Frontmatter data (opaque storage)
    pub frontmatter: Option<serde_json::Value>,
}

/// Link entity
#[derive(Debug, Clone)]
pub struct Link {
    pub source_note_id: NoteId,
    pub target_note_id: NoteId,
    
    /// Link position in source file (for Refactor positioning)
    pub range: TextRange,
    
    /// Link type: WikiLink, MarkdownLink, etc.
    pub kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkKind {
    WikiLink,     // [[foo]]
    MarkdownLink, // [foo](foo.md)
}