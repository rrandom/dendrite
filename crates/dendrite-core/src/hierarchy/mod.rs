use std::path::Path;

use crate::model::{Note, NoteId, NoteKey, ResolverId};

pub mod dendron;

/// Defines how to understand hierarchical relationships between notes
pub trait HierarchyResolver: Send + Sync {
    fn id(&self) -> ResolverId;
    fn resolve_note_key(&self, path: &Path, content: &str) -> NoteKey;
    fn resolve_link_key(&self, source: &NoteKey, raw: &str) -> NoteKey;
    /// Calculate the parent node ID for a given note
    /// Dendron implementation: "a.b" -> "a"
    /// Folder implementation: "a/b" -> "a"
    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey>;
    /// Calculate the display name for a given note
    /// Dendron implementation: "foo.bar" -> "bar"
    fn resolve_display_name(&self, note: &Note) -> String;
}

/// Basic resolver implementation (placeholder)
pub struct BasicResolver;

// impl HierarchyResolver for BasicResolver {
//     fn resolve_parent(&self, _note: &Note) -> Option<NoteId> {
//         None
//     }

//     fn resolve_display_name(&self, note: &Note) -> String {
//         note.id.clone()
//     }

//     fn resolve_id(&self, _root: &std::path::Path, path: &std::path::Path) -> Option<NoteId> {
//         // Basic implementation: use normalize_path_to_id
//         Some(crate::normalize_path_to_id(path))
//     }
// }

/// Dendron strategy implementation
#[allow(dead_code)]
pub struct DendronStrategy {
    // TODO: Implement Dendron-specific hierarchy resolution
}
