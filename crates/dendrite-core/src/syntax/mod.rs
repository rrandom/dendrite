use crate::{
    model::{Note, NoteKey, ResolverId},
};
use std::path::{Path, PathBuf};

mod dendron;

pub use dendron::DendronStrategy;

/// Syntax and Hierarchy rules for a specific vault format
pub trait SyntaxStrategy: Send + Sync {
    fn id(&self) -> ResolverId;
    fn note_key_from_path(&self, path: &Path, content: &str) -> NoteKey;
    fn note_key_from_link(&self, source: &NoteKey, raw: &str) -> NoteKey;
    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey>;
    fn resolve_display_name(&self, note: &Note) -> String;
    fn path_from_note_key(&self, key: &NoteKey) -> PathBuf;
    
    /// WikiLink format used by this syntax strategy
    fn wikilink_format(&self) -> WikiLinkFormat;
    
    /// Generate WikiLink text for refactoring
    fn format_wikilink(
        &self,
        target: &str,
        alias: Option<&str>,
        anchor: Option<&str>,
        is_embed: bool,
    ) -> String;
}

/// WikiLink format used by different syntax strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiLinkFormat {
    /// Dendron: [[alias|target#anchor]]
    AliasFirst,
    /// Obsidian: [[target#anchor|alias]]
    TargetFirst,
}
