use super::Workspace;
use crate::vfs::FileSystem;
use std::path::PathBuf;
use std::sync::Arc;

/// The Vault acts as the high-level Facade for the Dendrite Core.
///
/// # Architecture Decision: Action vs Query Separation
///
/// *   **Actions (Write/Refactor)**: Unified in `Vault`.
///     All operations that modify state (File Sync) or calculate changes (Refactoring)
///     SHOULD happen through methods on `Vault`. This ensures a single entry point for
///     business logic that may involve the FileSystem or Side Effects.
///
/// *   **Queries (Read)**: Access `vault.workspace` directly.
///     Read-only operations (resolving keys, looking up notes, graph traversal) DO NOT
///     need to be wrapped in `Vault`. Callers should access `vault.workspace` directly.
///     This avoids boilerplate and keeps the API surface clean.
pub struct Vault {
    pub workspace: Workspace,
    pub fs: Arc<dyn FileSystem>,
}

impl Vault {
    pub fn new(workspace: Workspace, fs: Arc<dyn FileSystem>) -> Self {
        Self { workspace, fs }
    }

    // ------------------------------------------------------------------------
    // File System Sync (Changes coming FROM disk)
    // ------------------------------------------------------------------------

    pub fn initialize(&mut self, root: PathBuf) -> Vec<PathBuf> {
        self.workspace.initialize(root, &*self.fs)
    }

    pub fn update_content(&mut self, path: PathBuf, content: &str) {
        self.workspace.update_file(path, content, &*self.fs);
    }

    pub fn delete_file(&mut self, path: &PathBuf) {
        self.workspace.delete_file(path, &*self.fs);
    }

    pub fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        self.workspace
            .rename_file(old_path, new_path, content, &*self.fs);
    }

    // ------------------------------------------------------------------------
    // Refactoring (Changes GOING TO disk)
    // ------------------------------------------------------------------------

    pub fn rename_note(
        &self,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        self.workspace.rename_note(self, old_key, new_key)
    }

    pub fn move_note(
        &self,
        old_path: &std::path::Path,
        new_path: std::path::PathBuf,
    ) -> Option<crate::refactor::model::EditPlan> {
        self.workspace.move_note(self, old_path, new_path)
    }

    pub fn rename_hierarchy(
        &self,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        self.workspace.rename_hierarchy(self, old_key, new_key)
    }

    pub fn split_note(
        &self,
        source_path: &std::path::Path,
        selection: crate::model::TextRange,
        new_note_title: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        self.workspace
            .split_note(self, source_path, selection, new_note_title)
    }

    pub fn audit(&self) -> crate::refactor::model::EditPlan {
        self.workspace.audit()
    }
}

impl crate::refactor::model::ContentProvider for Vault {
    fn get_content(&self, uri: &str) -> Option<String> {
        self.fs.read_to_string(&std::path::PathBuf::from(uri)).ok()
    }
}
