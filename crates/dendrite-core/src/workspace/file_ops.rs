use super::indexer::Indexer;
use super::Workspace;
use std::path::PathBuf;

impl Workspace {
    pub fn on_file_open(&mut self, path: PathBuf, text: String) {
        self.update_file(&path, &text);
    }

    pub fn on_file_changed(&mut self, path: PathBuf, new_text: String) {
        self.update_file(&path, &new_text);
    }

    pub fn on_file_rename(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        let mut indexer = Indexer::new(self);
        indexer.rename_file(old_path, new_path, content);
    }

    pub fn on_file_delete(&mut self, path: PathBuf) {
        let mut indexer = Indexer::new(self);
        indexer.delete_file(&path);
    }

    pub fn update_file(&mut self, file_path: &PathBuf, content: &str) {
        let mut indexer = Indexer::new(self);
        indexer.update_content(file_path.clone(), content);
    }

    pub fn index_files(&mut self, files: Vec<PathBuf>) {
        let mut indexer = Indexer::new(self);
        for path in files {
            indexer.index_file(path);
        }
    }
}
