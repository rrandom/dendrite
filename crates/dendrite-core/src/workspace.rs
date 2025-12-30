use std::path::PathBuf;
use walkdir::WalkDir;

use crate::hierarchy::HierarchyResolver;
use crate::identity::IdentityRegistry;
use crate::model::{Note, NoteId, NoteKey};
use crate::store::Store;

pub struct Workspace {
    pub root: PathBuf,
    pub resolver: Box<dyn HierarchyResolver>,
    pub identity: Box<dyn IdentityRegistry>,
    pub store: Store,
}

impl Workspace {
    pub fn new(
        root: PathBuf,
        resolver: Box<dyn HierarchyResolver>,
        identity: Box<dyn IdentityRegistry>,
    ) -> Self {
        Self {
            root,
            resolver,
            identity,
            store: Store::new(),
        }
    }

    pub fn on_file_open(&mut self, path: PathBuf, content: &str) {
        todo!()
    }

    pub fn on_file_change(&mut self, path: PathBuf, content: &str) {
        todo!()
    }

    pub fn on_file_rename(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        todo!()
    }

    pub fn on_file_delete(&mut self, path: PathBuf) {
        todo!()
    }

    pub fn note_by_path(&self, path: &PathBuf) -> Option<&Note> {
        todo!();
    }

    pub fn backlinks_of(&self, id: &NoteId) -> Vec<NoteId> {
        todo!()
    }

    pub fn all_notes(&self) -> Vec<&Note> {
        todo!()
    }

    /// Rename a note (semantic rename)
    pub fn rename_note(&mut self, id: &NoteId, new_key: NoteKey) {
        todo!();
    }

    /// Move a note to a new path
    pub fn move_note(&mut self, id: &NoteId, new_path: PathBuf) {
        todo!()
    }

    pub fn initialize(&mut self) -> Vec<PathBuf> {
        let mut md_files = Vec::new();

        for entry in WalkDir::new(&self.root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        md_files.push(path.to_path_buf());
                    }
                }
            }
        }

        // Parse and store all scanned files
        for file_path in &md_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                self.update_file(file_path, &content);
            }
        }

        md_files
    }

    pub fn update_file(&mut self, file_path: &PathBuf, content: &str) {
        todo!()
    }
}
