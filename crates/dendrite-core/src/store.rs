use crate::model::{Link, Note, NoteId};
use std::collections::HashMap;
use std::path::PathBuf;

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

    pub fn find_children(&self, parent_id: &str) -> Vec<&Note> {
        let prefix = format!("{}.", parent_id);
        self.notes.values()
            .filter(|n| {
                n.id.starts_with(&prefix) && 
                !n.id[prefix.len()..].contains('.')
            })
            .collect()
    }

    pub fn upsert_note(&mut self, note: Note) {
        let id = note.id.clone();
        
        if let Some(path) = &note.path {
            self.path_map.insert(path.clone(), id.clone());
        }
        self.notes.insert(id, note);
    }

    
    pub fn remove_note(&mut self, note_id: &str) {
        if let Some(note) = self.notes.remove(note_id) {
            if let Some(path) = &note.path {
                self.path_map.remove(path);
            }
            // Remove from backlinks
            self.backlinks.remove(note_id);
            // Remove backlinks pointing to this note
            for backlinks in self.backlinks.values_mut() {
                backlinks.retain(|id| id != note_id);
            }
        }
    }

    pub fn get_note_id_by_path(&self, path: &PathBuf) -> Option<NoteId> {
        self.path_map.get(path).cloned()
    }

    pub fn update_backlinks(&mut self, source_id: &str, old_links: &[Link], new_links: &[Link]) {
        // 1. 移除旧的
        for link in old_links {
            if let Some(refs) = self.backlinks.get_mut(&link.target_note_id) {
                refs.retain(|id| id != source_id);
            }
        }
        // 2. 添加新的
        for link in new_links {
            self.backlinks
                .entry(link.target_note_id.clone())
                .or_default()
                .push(source_id.to_string());
        }
    }
}
