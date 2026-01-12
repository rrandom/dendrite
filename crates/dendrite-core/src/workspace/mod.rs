use std::path::PathBuf;
use std::sync::RwLock;

use crate::identity::IdentityRegistry;
use crate::store::Store;
use crate::semantic::SemanticModel;
pub mod vfs;

mod assembler;
mod hierarchy;
mod indexer;
mod queries;
mod vault;

#[cfg(test)]
mod tests;

use hierarchy::NoteTree;
pub use indexer::Indexer;
pub use vault::Vault;
pub use vfs::FileSystem;

pub struct Workspace {
    pub(crate) resolver: Box<dyn SemanticModel>,
    pub(crate) identity: IdentityRegistry,
    pub(crate) store: Store,
    pub(crate) tree_cache: RwLock<Option<NoteTree>>,
}

impl Workspace {
    pub fn new(resolver: Box<dyn SemanticModel>) -> Self {
        Self {
            resolver,
            identity: IdentityRegistry::new(),
            store: Store::new(),
            tree_cache: RwLock::new(None),
        }
    }

    pub fn initialize(&mut self, root: PathBuf, fs: &dyn FileSystem) -> Vec<PathBuf> {
        let mut indexer = Indexer::new(self, fs);
        indexer.full_index(root)
    }

    /// Resolve the Note Identifier (Key) for a given path.
    pub fn resolve_note_key(&self, path: &std::path::Path) -> Option<String> {
        let key = self.resolver.note_key_from_path(path, "");
        Some(key)
    }

    /// Initiate a Rename Refactoring from old_key to new_key.
    pub fn rename_note(
        &self,
        content_provider: &dyn crate::refactor::model::ContentProvider,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        // 1. Lookup ID from Key
        let note_id = self.identity.lookup(&old_key.to_string())?;

        // 2. Calculate New Path (Forward Calculation using SyntaxStrategy)
        let new_path = self.resolver.path_from_note_key(&new_key.to_string());

        // 3. Delegate to Core Refactor Engine
        crate::refactor::structural::calculate_structural_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.resolver.as_ref(),
            &note_id,
            new_path,
            new_key,
        )
    }

    /// Initiate a Move Refactoring from old_path to new_path.
    pub fn move_note(
        &self,
        content_provider: &dyn crate::refactor::model::ContentProvider,
        old_path: &std::path::Path,
        new_path: std::path::PathBuf,
    ) -> Option<crate::refactor::model::EditPlan> {
        // 1. Resolve ID from Old Path
        let note_id = self.store.note_id_by_path(&old_path.to_path_buf())?.clone();

        // 2. Resolve target Key from Target Path
        let new_key = self.resolver.note_key_from_path(&new_path, "");

        // 3. Delegate to Core Refactor Engine
        crate::refactor::structural::calculate_structural_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.resolver.as_ref(),
            &note_id,
            new_path,
            &new_key,
        )
    }
}
