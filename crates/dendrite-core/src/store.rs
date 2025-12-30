use crate::model::{Note, NoteId};
use std::collections::HashMap;
use std::path::PathBuf;

// In memory
pub struct Store {
    pub(crate) notes: HashMap<NoteId, Note>,
    pub(crate) path_map: HashMap<PathBuf, NoteId>,
    pub(crate) backlinks: HashMap<NoteId, Vec<NoteId>>,
}

impl Store {
    pub(crate) fn new() -> Self {
        Self {
            notes: HashMap::new(),
            path_map: HashMap::new(),
            backlinks: HashMap::new(),
        }
    }

    pub(crate) fn upsert_note(&mut self, note: Note) {
        let id = note.id.clone();

        if let Some(old_note) = self.notes.get(&id) {
            if let Some(old_path) = &old_note.path {
                if self.path_map.get(old_path) == Some(&id) {
                    self.path_map.remove(old_path);
                }
            }
        }

        if let Some(path) = &note.path {
            self.path_map.insert(path.clone(), id.clone());
        }
        self.notes.insert(id, note);
    }

    pub(crate) fn remove_note(&mut self, id: &NoteId) {
        if let Some(note) = self.notes.remove(id) {
            if let Some(path) = &note.path {
                self.path_map.remove(path);
            }
            self.backlinks.remove(id);
            for backlinks in self.backlinks.values_mut() {
                backlinks.retain(|backlink_id| backlink_id != id);
            }
        }
    }

    pub(crate) fn get_note(&self, id: &NoteId) -> Option<&Note> {
        self.notes.get(id)
    }

    pub(crate) fn bind_path(&mut self, path: PathBuf, id: NoteId) {
        self.path_map.insert(path, id);
    }

    pub(crate) fn update_path(&mut self, id: &NoteId, new_path: PathBuf) {
        if let Some(note) = self.notes.get_mut(id) {
            if let Some(old_path) = &note.path {
                if self.path_map.get(old_path) == Some(id) {
                    self.path_map.remove(old_path);
                }
            }

            note.path = Some(new_path.clone());
            self.path_map.insert(new_path, id.clone());
        }
    }

    pub(crate) fn note_id_by_path(&self, path: &PathBuf) -> Option<&NoteId> {
        self.path_map.get(path)
    }

    /// Replace all outgoing links of a note
    pub(crate) fn set_outgoing_links(&mut self, source: &NoteId, targets: Vec<NoteId>) {
        let old_targets: Vec<NoteId> = self
            .notes
            .get(source)
            .map(|note| note.links.iter().map(|link| link.target.clone()).collect())
            .unwrap_or_default();

        for old_target in &old_targets {
            if let Some(backlinks) = self.backlinks.get_mut(old_target) {
                backlinks.retain(|backlink_id| backlink_id != source);
            }
        }

        for target in &targets {
            let backlinks = self
                .backlinks
                .entry(target.clone())
                .or_insert_with(Vec::new);
            if !backlinks.contains(source) {
                backlinks.push(source.clone());
            }
        }

        if let Some(note) = self.notes.get_mut(source) {
            if note.links.len() == targets.len() {
                for (link, target) in note.links.iter_mut().zip(targets.iter()) {
                    link.target = target.clone();
                }
            } else {
                use crate::model::{Link, LinkKind, Point, TextRange};
                note.links = targets
                    .into_iter()
                    .map(|target| Link {
                        target,
                        range: TextRange {
                            start: Point { line: 0, col: 0 },
                            end: Point { line: 0, col: 0 },
                        },
                        kind: LinkKind::WikiLink,
                    })
                    .collect();
            }
        }
    }

    /// Get backlinks (incoming edges)
    pub(crate) fn backlinks_of(&self, id: &NoteId) -> Vec<NoteId> {
        self.backlinks.get(id).cloned().unwrap_or_default()
    }

    pub(crate) fn all_notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.values()
    }
}
