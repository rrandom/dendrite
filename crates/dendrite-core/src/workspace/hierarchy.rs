use std::collections::HashMap;

use crate::model::{NoteId, NoteKey, TreeView};

use super::Workspace;

/// Internal tree structure for hierarchy
/// Uses NoteId for stable references
#[derive(Clone)]
pub(crate) struct NoteTree {
    pub(crate) root_nodes: Vec<NoteId>,
    pub(crate) children: HashMap<NoteId, Vec<NoteId>>,
    #[allow(dead_code)] // Reserved for future use (e.g., navigating up the tree)
    pub(crate) parent: HashMap<NoteId, NoteId>,
}

impl Workspace {
    /// Build the tree structure from all notes
    pub(crate) fn build_tree(&self) -> NoteTree {
        let mut root_nodes = Vec::new();
        let mut children: HashMap<NoteId, Vec<NoteId>> = HashMap::new();
        let mut parent: HashMap<NoteId, NoteId> = HashMap::new();

        // Build parent-child relationships
        for note in self.store.all_notes() {
            let note_id = &note.id;

            // Get note key
            let note_key = self
                .identity
                .key_of(note_id)
                .map(|(_, key)| key)
                .or_else(|| {
                    note.path.as_ref().and_then(|path| {
                        // Try to get key from path (for notes that haven't been indexed yet)
                        Some(self.resolver.note_key_from_path(path, ""))
                    })
                });

            let Some(note_key) = note_key else {
                // Skip notes without key
                continue;
            };

            // Get parent key
            let parent_key = self.resolver.resolve_parent(&note_key);

            if let Some(parent_key) = parent_key {
                // Try to find parent NoteId
                if let Some(parent_id) = self.identity.lookup(&parent_key) {
                    // Parent exists, establish relationship
                    children
                        .entry(parent_id.clone())
                        .or_insert_with(Vec::new)
                        .push(note_id.clone());
                    parent.insert(note_id.clone(), parent_id);
                } else {
                    // Parent doesn't exist (Ghost Node), this is a root node
                    root_nodes.push(note_id.clone());
                }
            } else {
                // No parent, this is a root node
                root_nodes.push(note_id.clone());
            }
        }

        NoteTree {
            root_nodes,
            children,
            parent,
        }
    }

    /// Get the tree structure (builds if needed)
    /// Returns a cloned copy of the tree to avoid holding the lock
    pub(crate) fn tree(&self) -> NoteTree {
        // Check if cache exists
        {
            let cache = self.tree_cache.read().unwrap();
            if let Some(tree) = cache.as_ref() {
                return tree.clone();
            }
        }

        // Build tree and cache it
        let tree = self.build_tree();
        {
            let mut cache = self.tree_cache.write().unwrap();
            *cache = Some(tree.clone());
        }
        tree
    }

    /// Invalidate the tree cache
    pub(crate) fn invalidate_tree(&self) {
        let mut cache = self.tree_cache.write().unwrap();
        *cache = None;
    }

    /// Get tree view for LSP protocol
    pub fn get_tree_view(&self) -> Vec<TreeView> {
        let tree = self.tree();

        // Build tree view from root nodes
        tree.root_nodes
            .iter()
            .filter_map(|root_id| self.build_tree_view_node(root_id, &tree))
            .collect()
    }

    /// Helper function to build TreeView from NoteId (recursive)
    fn build_tree_view_node(&self, note_id: &NoteId, tree: &NoteTree) -> Option<TreeView> {
        let note = self.store.get_note(note_id)?;

        // Get note key
        let note_key = self
            .identity
            .key_of(note_id)
            .map(|(_, key)| key)
            .or_else(|| {
                note.path
                    .as_ref()
                    .map(|path| self.resolver.note_key_from_path(path, ""))
            });

        // Get path as URI string
        let path_uri = note.path.as_ref().and_then(|path| {
            // Convert PathBuf to URI string
            path.to_str().map(|s| {
                if cfg!(windows) {
                    format!("file:///{}", s.replace('\\', "/"))
                } else {
                    format!("file://{}", s)
                }
            })
        });

        // Get children (recursive)
        let children = tree
            .children
            .get(note_id)
            .map(|child_ids| {
                child_ids
                    .iter()
                    .filter_map(|child_id| self.build_tree_view_node(child_id, tree))
                    .collect()
            })
            .unwrap_or_default();

        // Convert NoteId to string (UUID)
        let id_string = note_id.0.to_string();

        Some(TreeView {
            note: crate::model::NoteRef {
                id: id_string,
                key: note_key,
                path: path_uri,
                title: note.title.clone(),
            },
            children,
        })
    }

    /// fill missing hierarchy levels
    pub(crate) fn fill_missing_hierarchy_levels(&mut self) {
        // collect all real note keys
        let real_note_keys: std::collections::HashSet<NoteKey> = self
            .store
            .all_notes()
            .filter_map(|note| {
                // only process notes with path
                if note.path.is_some() {
                    self.identity.key_of(&note.id).map(|(_, key)| key)
                } else {
                    None
                }
            })
            .collect();

        // collect all virtual note keys
        let mut virtual_keys = std::collections::HashSet::new();

        for note_key in &real_note_keys {
            // recursively find all missing parent nodes
            let mut current_key = note_key.clone();
            while let Some(parent_key) = self.resolver.resolve_parent(&current_key) {
                // if parent node does not exist (neither in real notes, nor in collected virtual keys)
                if !real_note_keys.contains(&parent_key) && !virtual_keys.contains(&parent_key) {
                    virtual_keys.insert(parent_key.clone());
                }
                current_key = parent_key;
            }
        }

        // create virtual notes for each virtual key
        for virtual_key in virtual_keys {
            // get or create NoteId
            let virtual_id = self.identity.get_or_create(&virtual_key);

            // check if it already exists (maybe created before)
            if self.store.get_note(&virtual_id).is_some() {
                continue;
            }

            // create virtual note (no path, no content)
            let virtual_note = crate::model::Note {
                id: virtual_id.clone(),
                path: None, // virtual note has no actual file
                title: None,
                frontmatter: None,
                links: Vec::new(),
                headings: Vec::new(),
                blocks: Vec::new(),
                digest: None,
            };

            // add to store
            self.store.upsert_note(virtual_note);
        }
    }
}
