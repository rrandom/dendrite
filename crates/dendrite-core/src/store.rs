use crate::model::{Note, NoteId};
use std::collections::HashMap;
use std::path::PathBuf;

// In memory
pub struct Store {
    pub notes: HashMap<NoteId, Note>,
    pub path_map: HashMap<PathBuf, NoteId>,
    pub backlinks: HashMap<NoteId, Vec<NoteId>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            notes: HashMap::new(),
            path_map: HashMap::new(),
            backlinks: HashMap::new(),
        }
    }

    pub fn upsert_note(&mut self, note: Note) {
        let id = note.id.clone();

        if let Some(path) = &note.path {
            self.path_map.insert(path.clone(), id.clone());
        }
        self.notes.insert(id, note);
    }

    pub fn remove_note(&mut self, id: &NoteId) {
        if let Some(note) = self.notes.remove(id) {
            if let Some(path) = &note.path {
                self.path_map.remove(path);
            }
            // Remove from backlinks
            self.backlinks.remove(id);
            // Remove backlinks pointing to this note
            for backlinks in self.backlinks.values_mut() {
                backlinks.retain(|id| id != id);
            }
        }
    }

    pub fn get_note(&self, id: &NoteId) -> Option<&Note> {
        self.notes.get(id)
    }

    pub fn get_note_mut(&mut self, id: &NoteId) -> Option<&mut Note> {
        self.notes.get_mut(id)
    }

    pub fn bind_path(&mut self, path: PathBuf, id: NoteId) {
        self.path_map.insert(path, id);
    }

    pub fn unbind_path(&mut self, path: &PathBuf) {
        self.path_map.remove(path);
    }


    pub fn note_id_by_path(&self, path: &PathBuf) -> Option<&NoteId> {
        self.path_map.get(path)
    }

    /// Replace all outgoing links of a note
    pub fn set_outgoing_links(
        &mut self,
        source: &NoteId,
        targets: Vec<NoteId>,
    ) {
        todo!()
    }

    /// Get backlinks (incoming edges)
    pub fn backlinks_of(&self, id: &NoteId) -> Vec<NoteId> {
        todo!();
    }

    pub fn all_notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.values()
    }
}
