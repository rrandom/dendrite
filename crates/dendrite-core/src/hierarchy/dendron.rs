use std::path::Path;

use crate::{
    hierarchy::HierarchyResolver,
    model::{NoteKey, ResolverId},
    normalize_path_to_id,
};

pub struct DendronStrategy;

impl HierarchyResolver for DendronStrategy {
    fn id(&self) -> ResolverId {
        ResolverId("Dendron")
    }

    fn resolve_note_key(&self, path: &Path, _: &str) -> NoteKey {
        let note_key = normalize_path_to_id(path);
        note_key
    }

    fn resolve_link_key(&self, source: &NoteKey, raw: &str) -> NoteKey {
        let link_key = normalize_path_to_id(&Path::new(raw));
        link_key
    }
    fn resolve_display_name(&self, note: &crate::model::Note) -> String {
        note.title.clone().unwrap_or_default()
    }
    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey> {
        let parent_key = note.split('.').nth(0)?;
        Some(parent_key.to_string())
    }
}
