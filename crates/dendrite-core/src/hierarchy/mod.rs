use std::path::{Path, PathBuf};
use crate::{model::{Note, NoteKey, ResolverId}, normalize_path_to_id};

/// path/link/text => notekey
pub trait HierarchyResolver: Send + Sync {
    fn id(&self) -> ResolverId;
    fn note_key_from_path(&self, path: &Path, content: &str) -> NoteKey;
    fn note_key_from_link(&self, source: &NoteKey, raw: &str) -> NoteKey;
    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey>;
    fn resolve_display_name(&self, note: &Note) -> String;
    fn path_from_note_key(&self, key: &NoteKey) -> PathBuf;
}

pub struct DendronStrategy;

impl DendronStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl HierarchyResolver for DendronStrategy {
    fn id(&self) -> ResolverId {
        ResolverId("Dendron")
    }
    fn note_key_from_path(&self, path: &Path, _: &str) -> NoteKey {
        let note_key = normalize_path_to_id(path);
        note_key
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
        let parent_key = note.split('.').nth(0)?;
        Some(parent_key.to_string())
    }
    fn path_from_note_key(&self, key: &NoteKey) -> std::path::PathBuf {
        Path::new(&key).with_extension("md")
    }
}