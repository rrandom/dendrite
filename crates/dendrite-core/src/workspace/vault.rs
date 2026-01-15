use super::Workspace;
use crate::vfs::FileSystem;
use std::path::PathBuf;
use std::sync::Arc;

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
}

impl crate::refactor::model::ContentProvider for Vault {
    fn get_content(&self, uri: &str) -> Option<String> {
        self.fs.read_to_string(&std::path::PathBuf::from(uri)).ok()
    }
}
