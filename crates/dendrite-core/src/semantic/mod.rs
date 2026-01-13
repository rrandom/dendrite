use crate::model::{Note, NoteKey, ResolverId};
use std::path::{Path, PathBuf};

mod dendron;

pub use dendron::DendronModel;

/// Semantic Model: The brain of the vault
/// Defines how raw files are interpreted as structured knowledge
pub trait SemanticModel: Send + Sync {
    fn id(&self) -> ResolverId;

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

    /// WikiLink format used by this model
    fn wikilink_format(&self) -> WikiLinkFormat;

    /// Generate WikiLink text for refactoring
    fn format_wikilink(
        &self,
        target: &str,
        alias: Option<&str>,
        anchor: Option<&str>,
        is_embed: bool,
    ) -> String;

    // --- Extension Points ---

    /// Supported file extensions (e.g., &["md", "org"])
    fn supported_extensions(&self) -> &[&str];

    /// Optional: parsing hints for the engine
    ///
    /// This is a placeholder for future extensions, such as:
    /// - Logseq: treat all bullets as blocks
    /// - OrgMode: different heading depth rules
    /// - Frontmatter: custom delimiters
    fn parsing_hints(&self) -> Option<()> {
        None
    }
}

/// WikiLink format used by different syntax strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiLinkFormat {
    /// Dendron: [[alias|target#anchor]]
    AliasFirst,
    /// Obsidian: [[target#anchor|alias]]
    TargetFirst,
}
