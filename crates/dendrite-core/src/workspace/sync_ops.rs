use std::path::PathBuf;

use crate::vfs::FileSystem;

use super::{Indexer, Workspace};

/// File System Integration (Input)
/// These methods are triggered by file system events (File Watcher) or initialization.
/// They are responsible for KEEPING the workspace in sync with what is on disk.
/// They DO NOT modify files on disk, they only update the in-memory state (Store/Index).
impl Workspace {
    pub fn initialize(
        &mut self,
        root: PathBuf,
        fs: &dyn FileSystem,
    ) -> (Vec<PathBuf>, crate::workspace::indexer::IndexingStats) {
        let mut indexer = Indexer::new(self, fs);
        indexer.full_index(root)
    }

    pub fn update_file(&mut self, path: PathBuf, content: &str, fs: &dyn FileSystem) {
        let mut indexer = Indexer::new(self, fs);
        indexer.update_content(path, content);
    }

    pub fn delete_file(&mut self, path: &PathBuf, fs: &dyn FileSystem) {
        let mut indexer = Indexer::new(self, fs);
        indexer.delete_file(path);
    }

    pub fn rename_file(
        &mut self,
        old_path: PathBuf,
        new_path: PathBuf,
        content: &str,
        fs: &dyn FileSystem,
    ) {
        let mut indexer = Indexer::new(self, fs);
        indexer.rename_file(old_path, new_path, content);
    }
}
