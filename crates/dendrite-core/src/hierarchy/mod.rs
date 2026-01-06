use crate::{
    model::{Note, NoteKey, ResolverId},
    normalize_path_to_id,
};
use std::path::{Path, PathBuf};

/// Syntax and Hierarchy rules for a specific vault format
pub trait SyntaxStrategy: Send + Sync {
    fn id(&self) -> ResolverId;
    fn note_key_from_path(&self, path: &Path, content: &str) -> NoteKey;
    fn note_key_from_link(&self, source: &NoteKey, raw: &str) -> NoteKey;
    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey>;
    fn resolve_display_name(&self, note: &Note) -> String;
    fn path_from_note_key(&self, key: &NoteKey) -> PathBuf;
}

pub struct DendronStrategy {
    root: PathBuf,
}

impl DendronStrategy {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl SyntaxStrategy for DendronStrategy {
    fn id(&self) -> ResolverId {
        ResolverId("Dendron")
    }
    fn note_key_from_path(&self, path: &Path, _: &str) -> NoteKey {
        // Dendron note key is just the filename without .md extension
        // e.g., "foo.bar.md" -> "foo.bar"
        if let Some(file_name) = path.file_stem() {
            file_name.to_string_lossy().to_string()
        } else {
            // Fallback to normalize_path_to_id if no file stem
            normalize_path_to_id(path)
        }
    }
    fn note_key_from_link(&self, source: &NoteKey, raw: &str) -> NoteKey {
        let link_path = Path::new(raw);
        if link_path.is_absolute() || raw.contains('/') || raw.contains('\\') {
            normalize_path_to_id(link_path)
        } else {
            let source_path = Path::new(source);
            if let Some(parent) = source_path.parent() {
                let resolved_path = parent.join(raw);
                normalize_path_to_id(&resolved_path)
            } else {
                normalize_path_to_id(link_path)
            }
        }
    }
    fn resolve_display_name(&self, note: &crate::model::Note) -> String {
        note.title.clone().unwrap_or_default()
    }
    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey> {
        // For Dendron: "foo.bar.baz" -> "foo.bar", "foo.bar" -> "foo", "foo" -> None
        let parts: Vec<&str> = note.split('.').collect();
        if parts.len() <= 1 {
            // No parent (root level or single part)
            return None;
        }
        // Return all parts except the last one, joined by '.'
        Some(parts[..parts.len() - 1].join("."))
    }
    fn path_from_note_key(&self, key: &NoteKey) -> std::path::PathBuf {
        // Generate full path: root / "key.md"
        // e.g., root = "/workspace", key = "foo.bar" -> "/workspace/foo.bar.md"
        self.root.join(format!("{}.md", key))
    }
}
