use std::path::PathBuf;

use crate::model::Point;
use crate::model::{Link, Note, NoteKey, TextRange};

use crate::slugify_heading;

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

    /// Resolve a link's anchor to a specific range within the target note
    pub fn resolve_link_anchor(&self, link: &Link) -> Option<TextRange> {
        let note = self.store.get_note(&link.target)?;
        let anchor = link.anchor.as_ref()?;

        if anchor.starts_with('^') {
            // Block anchor
            let block_id = &anchor[1..];
            note.blocks
                .iter()
                .find(|b| b.id == block_id)
                .map(|b| b.range)
        } else {
            // Heading anchor - use slugified comparison
            note.headings
                .iter()
                .find(|h| slugify_heading(&h.text) == *anchor)
                .map(|h| h.range)
        }
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
                    let display_name = self.model.resolve_display_name(note);
                    (key, display_name)
                })
            })
            .collect()
    }
}
