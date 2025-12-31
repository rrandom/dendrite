use std::path::PathBuf;

use crate::model::Point;
use crate::model::{Link, Note, NoteKey};

use super::Workspace;

impl Workspace {
    pub fn note_by_path(&self, path: &PathBuf) -> Option<&Note> {
        let id = self.store.note_id_by_path(path)?;
        self.store.get_note(id)
    }

    /// Find a link at the given position in a document
    pub fn find_link_at_position(&self, path: &PathBuf, position: Point) -> Option<&Link> {
        let note = self.note_by_path(path)?;
        note.links.iter().find(|link| {
            let range = link.range;
            // Check if position is within the link range
            (range.start.line < position.line
                || (range.start.line == position.line && range.start.col <= position.col))
                && (position.line < range.end.line
                    || (position.line == range.end.line && position.col <= range.end.col))
        })
    }

    /// Get the file path for a link's target
    pub fn get_link_target_path(&self, link: &Link) -> Option<PathBuf> {
        self.store
            .get_note(&link.target)
            .and_then(|note| note.path.clone())
    }

    pub fn backlinks_of(&self, path: &PathBuf) -> Vec<PathBuf> {
        let Some(id) = self.store.note_id_by_path(&path) else {
            return vec![];
        };

        self.store
            .backlinks_of(&id)
            .iter()
            .filter_map(|backlink_id| {
                self.store
                    .get_note(backlink_id)
                    .and_then(|note| note.path.clone())
            })
            .collect()
    }

    pub fn all_notes(&self) -> Vec<&Note> {
        self.store.all_notes().collect()
    }

    /// Get all note keys for completion
    /// Returns a vector of (note_key, display_name) tuples
    pub fn all_note_keys(&self) -> Vec<(NoteKey, String)> {
        self.store
            .all_notes()
            .filter_map(|note| {
                self.identity.key_of(&note.id).map(|(_, key)| {
                    let display_name = self.resolver.resolve_display_name(note);
                    (key, display_name)
                })
            })
            .collect()
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
    pub fn move_note(&mut self, old_path: PathBuf, new_path: PathBuf) {
        let Some(id) = self.store.note_id_by_path(&old_path).cloned() else {
            let Ok(content) = std::fs::read_to_string(&new_path) else {
                return;
            };
            self.update_file(&new_path, &content);
            return;
        };

        let Ok(content) = std::fs::read_to_string(&new_path) else {
            return;
        };

        let Some((_, old_key)) = self.identity.key_of(&id) else {
            self.update_file(&new_path, &content);
            return;
        };

        let new_key = self.resolver.note_key_from_path(&new_path, &content);

        if old_key != new_key {
            let _ = self.identity.rebind(&old_key, &new_key);
        }

        let note = self.parse_note(&content, &new_path, &id);
        let targets: Vec<crate::model::NoteId> =
            note.links.iter().map(|link| link.target.clone()).collect();
        self.store.upsert_note(note);
        self.store.bind_path(new_path, id.clone());
        self.store.set_outgoing_links(&id, targets);
    }
}
