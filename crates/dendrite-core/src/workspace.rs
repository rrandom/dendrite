use std::path::PathBuf;
use walkdir::WalkDir;

use crate::hierarchy::HierarchyResolver;
use crate::identity::IdentityRegistry;
use crate::model::{Link, Note, NoteId, NoteKey};
use crate::parser::parse_markdown;
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

    pub fn on_file_open(&mut self, path: PathBuf, text: String) {
        let note_key = self.resolver.note_key_from_path(&path, &text);
        let note_id = self.identity.get_or_create(&note_key);
        let note = self.parse_note(&text, &path, &note_id);
        self.store.upsert_note(note);
        self.store.bind_path(path, note_id);
    }

    pub fn on_file_changed(&mut self, path: PathBuf, new_text: String) {
        self.on_file_open(path, new_text)
    }

    pub fn on_file_rename(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        todo!()
    }

    pub fn on_file_delete(&mut self, path: PathBuf) {
        let Some(id) = self.store.note_id_by_path(&path) else {
            return;
        };
        // self.store.remove_note(id);
    }

    pub fn note_by_path(&self, path: &PathBuf) -> Option<&Note> {
        todo!();
    }

    pub fn backlinks_of(&self, path: &PathBuf) -> Vec<NoteId> {
        let Some(id) = self.store.note_id_by_path(&path) else {
            return vec![];
        };

        self.store.backlinks_of(&id)
    }

    pub fn all_notes(&self) -> Vec<&Note> {
        self.store.all_notes().collect()
    }

    /// Rename a note (semantic rename)
    pub fn rename_note(&mut self, old_path: PathBuf, new_key: NoteKey) {
        let old_key = self.resolver.note_key_from_path(&old_path, "");

        let Some(id) = self.identity.rebind(&old_key, &new_key) else {
            return;
        };

        let new_path = self.resolver.path_from_note_key(&new_key);

        self.store.update_path(&id, new_path);
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

    pub fn index_files(&mut self, files: Vec<PathBuf>) {
        for path in files {
            self.index_file(path);
        }
    }

    fn index_file(&mut self, path: PathBuf) {
        let content = std::fs::read_to_string(&path).unwrap();
        let note_key = self.resolver.note_key_from_path(&path, &content);
        let note_id = self.identity.get_or_create(&note_key);
        let note = self.parse_note(&content, &path, &note_id);
        self.store.upsert_note(note);
        self.store.bind_path(path, note_id);
    }

    pub fn parse_note(&mut self, content: &str, path: &PathBuf, note_id: &NoteId) -> Note {
        let parse_result = parse_markdown(content);
        Note {
            id: note_id.clone(),
            path: Some(path.clone()),
            title: parse_result.title,
            frontmatter: parse_result.frontmatter,
            links: parse_result
                .links
                .iter()
                .map(|link| Link {
                    target: self.identity.get_or_create(&link.target.clone()),
                    range: link.range,
                    kind: link.kind.clone(),
                })
                .collect(),
            headings: parse_result.headings,
        }
    }
}
