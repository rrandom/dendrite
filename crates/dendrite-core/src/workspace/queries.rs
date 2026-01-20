use std::path::PathBuf;

use crate::model::Point;
use crate::model::{Link, Note, NoteKey, TextRange};

use crate::slugify_heading;

use super::Workspace;

impl Workspace {
    /// Resolve the Note Identifier (Key) for a given path.
    pub fn resolve_note_key(&self, path: &std::path::Path) -> Option<String> {
        let key = self.model.note_key_from_path(path, "");
        Some(key)
    }

    /// Get the root path of the workspace
    pub fn root(&self) -> &std::path::Path {
        self.model.root()
    }

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

        // Handle reserved anchors
        match anchor.as_str() {
            "^begin" => {
                // Reference from document start to first heading
                let end = note
                    .headings
                    .first()
                    .map(|h| h.range.start)
                    .unwrap_or(Point {
                        line: u32::MAX,
                        col: 0,
                    });
                return Some(TextRange {
                    start: Point { line: 0, col: 0 },
                    end,
                });
            }
            "^end" => {
                // Reference from last element to document end
                let start = note
                    .headings
                    .last()
                    .map(|h| h.range.end)
                    .or_else(|| note.blocks.last().map(|b| b.range.end))
                    .unwrap_or(Point { line: 0, col: 0 });
                return Some(TextRange {
                    start,
                    end: Point {
                        line: u32::MAX,
                        col: 0,
                    },
                });
            }
            _ => {}
        }

        // Standard anchor resolution
        if let Some(block_id) = anchor.strip_prefix('^') {
            // Block anchor - strip ^ prefix before comparing
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
        let Some(id) = self.store.note_id_by_path(path) else {
            return vec![];
        };

        self.store
            .backlinks_of(id)
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
                self.identity.key_of(&note.id).map(|key| {
                    let display_name = self.model.resolve_display_name(note);
                    (key, display_name)
                })
            })
            .collect()
    }

    /// Lookup a note by its Note Key
    pub fn lookup_note(&self, key: &str) -> Option<&Note> {
        self.identity
            .lookup(&key.to_string())
            .and_then(|id| self.store.get_note(&id))
    }

    // =========================================================================
    // Facade APIs: Unified access to SemanticModel, IdentityRegistry, and Store
    // =========================================================================

    /// Get the NoteKey for a given Note
    pub fn key_of_note(&self, note: &Note) -> Option<NoteKey> {
        self.identity.key_of(&note.id)
    }

    /// Calculate the file path for a NoteKey (does not check if file exists)
    pub fn path_for_key(&self, key: &NoteKey) -> std::path::PathBuf {
        self.model.path_from_note_key(key)
    }

    /// Get the parent NoteKey (Dendron hierarchy)
    pub fn parent_of(&self, key: &NoteKey) -> Option<NoteKey> {
        self.model.resolve_parent(key)
    }

    /// Check if `candidate` is a descendant of `parent`
    pub fn is_descendant(&self, candidate: &NoteKey, parent: &NoteKey) -> bool {
        self.model.is_descendant(candidate, parent)
    }

    /// Get the display name for a Note
    pub fn display_name(&self, note: &Note) -> String {
        self.model.resolve_display_name(note)
    }

    /// Format a WikiLink according to the semantic model
    pub fn format_wikilink(&self, target: &str, alias: Option<&str>) -> String {
        self.model.format_wikilink(target, alias, None, false)
    }

    /// Audit the entire workspace for reference graph health.
    pub fn audit(&self) -> crate::mutation::model::EditPlan {
        crate::analysis::audit::calculate_audit_diagnostics(&self.store, self.model.as_ref())
    }
}
