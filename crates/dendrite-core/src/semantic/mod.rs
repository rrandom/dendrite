use crate::model::{ModelId, Note, NoteKey};
use std::path::{Path, PathBuf};

mod dendron;

pub use dendron::DendronModel;

/// Semantic Model: The brain of the vault
/// Defines how raw files are interpreted as structured knowledge
pub trait SemanticModel: Send + Sync {
    fn id(&self) -> ModelId;

    /// Get the root path of the workspace
    fn root(&self) -> &Path;

    // --- Identity & Resolution ---

    fn note_key_from_path(&self, path: &Path, content: &str) -> NoteKey;
    fn note_key_from_link(&self, source: &NoteKey, raw: &str) -> NoteKey;
    fn path_from_note_key(&self, key: &NoteKey) -> PathBuf;

    // --- Hierarchy ---

    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey>;

    /// Check if `candidate` is a descendant of `parent` in the hierarchy
    fn is_descendant(&self, candidate: &NoteKey, parent: &NoteKey) -> bool;

    /// Calculate new key for a descendant when its parent is renamed/moved
    fn reparent_key(&self, key: &NoteKey, old_parent: &NoteKey, new_parent: &NoteKey) -> NoteKey;

    // --- Display & Formatting ---

    fn resolve_display_name(&self, note: &Note) -> String;

    /// Generate WikiLink text for refactoring
    fn format_wikilink(
        &self,
        target: &str,
        alias: Option<&str>,
        anchor: Option<&str>,
        is_embed: bool,
    ) -> String;

    /// Supported link kinds for this strategy (for parsing)
    fn supported_link_kinds(&self) -> Vec<crate::model::LinkKind> {
        vec![]
    }

    /// Link kinds that should be audited for health (broken links)
    /// Defaults to all supported link kinds.
    fn audited_link_kinds(&self) -> Vec<crate::model::LinkKind> {
        self.supported_link_kinds()
    }

    // --- Extension Points ---

    /// Supported file extensions (e.g., &["md", "org"])
    fn supported_extensions(&self) -> &[&str];

    /// Optional: parsing hints for the engine
    ///
    /// This is a placeholder for future extensions, such as:
    /// - Logseq: treat all bullets as blocks
    /// - OrgMode: different heading depth rules
    /// - Frontmatter: custom delimiters
    ///
    /// Generate initial content for a new note
    fn generate_new_note_content(&self, _key: &NoteKey) -> String {
        "# New Note".to_string()
    }
}
