use super::vfs::FileSystem;
use super::{Indexer, Workspace};
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

    pub fn initialize(&mut self, root: PathBuf) -> Vec<PathBuf> {
        let mut indexer = Indexer::new(&mut self.workspace, &*self.fs);
        indexer.full_index(root)
    }

    pub fn update_content(&mut self, path: PathBuf, content: &str) {
        let mut indexer = Indexer::new(&mut self.workspace, &*self.fs);
        indexer.update_content(path, content);
    }

    pub fn delete_file(&mut self, path: &PathBuf) {
        let mut indexer = Indexer::new(&mut self.workspace, &*self.fs);
        indexer.delete_file(path);
    }

    pub fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        let mut indexer = Indexer::new(&mut self.workspace, &*self.fs);
        indexer.rename_file(old_path, new_path, content);
    }

    pub fn rename_note(
        &self,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        self.workspace.rename_note(old_key, new_key)
    }
}
