use crate::graph::Graph;
use crate::model::{Note, NoteId};
use crate::trie::HierarchyTrie;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Store {
    /// Base storage: ID -> Note
    pub notes: HashMap<NoteId, Note>,

    /// Relationship graph: handles [[Link]] and Backlinks
    pub graph: Graph,

    /// Hierarchy tree: handles foo.bar.baz hierarchical relationships
    /// Core data structure for Dendron compatibility layer
    pub trie: HierarchyTrie,

    /// Path index: Path -> ID
    pub path_map: HashMap<PathBuf, NoteId>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            notes: HashMap::new(),
            graph: Graph{},
            trie: HierarchyTrie{},
            path_map: HashMap::new(),
        }
    }

    /// Insert a note into the store
    /// Store only handles atomic operations, not file I/O
    pub fn insert(&mut self, note: Note) {
        // TODO: Implement note insertion logic
        let _ = note;
    }
}
