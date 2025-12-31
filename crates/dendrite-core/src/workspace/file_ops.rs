use std::path::PathBuf;

use crate::model::{Link, Note, NoteId};
use crate::parser::parse_markdown;

use super::Workspace;

impl Workspace {
    pub fn on_file_open(&mut self, path: PathBuf, text: String) {
        self.update_file(&path, &text);
    }

    pub fn on_file_changed(&mut self, path: PathBuf, new_text: String) {
        self.update_file(&path, &new_text);
    }

    pub fn on_file_rename(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        let Some(old_id) = self.store.note_id_by_path(&old_path).cloned() else {
            self.update_file(&new_path, content);
            return;
        };

        let old_key = self
            .identity
            .key_of(&old_id)
            .map(|(_, key)| key)
            .unwrap_or_else(|| self.resolver.note_key_from_path(&old_path, content));

        let new_key = self.resolver.note_key_from_path(&new_path, content);

        if old_key != new_key {
            let _ = self.identity.rebind(&old_key, &new_key);
        }

        let note = self.parse_note(content, &new_path, &old_id);
        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.store.upsert_note(note);
        self.store.bind_path(new_path, old_id.clone());
        self.store.set_outgoing_links(&old_id, targets);

        // Key change affects tree structure
        if old_key != new_key {
            self.invalidate_tree();
        }
    }

    pub fn on_file_delete(&mut self, path: PathBuf) {
        let Some(id) = self.store.note_id_by_path(&path).cloned() else {
            return;
        };
        self.store.remove_note(&id);
        self.invalidate_tree();
    }

    pub fn update_file(&mut self, file_path: &PathBuf, content: &str) {
        let new_key = self.resolver.note_key_from_path(file_path, content);

        let note_id = if let Some(existing_id) = self.store.note_id_by_path(file_path) {
            let existing_id = existing_id.clone();

            if let Some((_, old_key)) = self.identity.key_of(&existing_id) {
                if old_key != new_key {
                    let _ = self.identity.rebind(&old_key, &new_key);
                }
            }

            existing_id
        } else {
            self.identity.get_or_create(&new_key)
        };

        let note = self.parse_note(content, file_path, &note_id);
        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.store.upsert_note(note);
        self.store.bind_path(file_path.clone(), note_id.clone());
        self.store.set_outgoing_links(&note_id, targets);

        // invalidate tree on file update (key might have changed, or new note added)
        self.invalidate_tree();
    }

    pub fn index_files(&mut self, files: Vec<PathBuf>) {
        for path in files {
            self.index_file(path);
        }
    }

    fn index_file(&mut self, path: PathBuf) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        self.update_file(&path, &content);
    }

    pub(crate) fn parse_note(&mut self, content: &str, path: &PathBuf, note_id: &NoteId) -> Note {
        let parse_result = parse_markdown(content);
        let source_key = self.resolver.note_key_from_path(path, content);

        Note {
            id: note_id.clone(),
            path: Some(path.clone()),
            title: parse_result.title,
            frontmatter: parse_result.frontmatter,
            links: parse_result
                .links
                .iter()
                .map(|link| {
                    let link_key = self.resolver.note_key_from_link(&source_key, &link.target);
                    Link {
                        target: self.identity.get_or_create(&link_key),
                        range: link.range,
                        kind: link.kind.clone(),
                    }
                })
                .collect(),
            headings: parse_result.headings,
        }
    }
}
