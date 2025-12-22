use crate::model::{Note, NoteId};

/// Defines how to understand hierarchical relationships between notes
pub trait HierarchyResolver {
    /// Calculate the parent node ID for a given note
    /// Dendron implementation: "a.b" -> "a"
    /// Folder implementation: "a/b" -> "a"
    fn resolve_parent(&self, note: &Note) -> Option<NoteId>;
    
    /// Calculate the display name for a given note
    /// Dendron implementation: "foo.bar" -> "bar"
    fn resolve_display_name(&self, note: &Note) -> String;
    
    /// Calculate Note ID from file path
    /// Dendron implementation: path/to/foo.bar.md -> "foo.bar"
    fn resolve_id(&self, root: &std::path::Path, path: &std::path::Path) -> Option<NoteId>;
}

/// Basic resolver implementation (placeholder)
pub struct BasicResolver;

impl HierarchyResolver for BasicResolver {
    fn resolve_parent(&self, _note: &Note) -> Option<NoteId> {
        None
    }
    
    fn resolve_display_name(&self, note: &Note) -> String {
        note.id.clone()
    }
    
    fn resolve_id(&self, _root: &std::path::Path, path: &std::path::Path) -> Option<NoteId> {
        // Basic implementation: use normalize_path_to_id
        Some(crate::normalize_path_to_id(path))
    }
}

/// Dendron strategy implementation
#[allow(dead_code)]
pub struct DendronStrategy {
    // TODO: Implement Dendron-specific hierarchy resolution
}
