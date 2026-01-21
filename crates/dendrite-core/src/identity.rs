use crate::model::{NoteId, NoteKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// NoteKey <=> NoteId
/// Concrete registry for managing unique identifiers for notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRegistry {
    key_to_id: HashMap<NoteKey, NoteId>,
    id_to_key: HashMap<NoteId, NoteKey>,
}

impl IdentityRegistry {
    pub fn new() -> Self {
        Self {
            key_to_id: HashMap::new(),
            id_to_key: HashMap::new(),
        }
    }

    #[allow(private_interfaces)]
    pub fn get_or_create(&mut self, key: &NoteKey) -> NoteId {
        if let Some(id) = self.key_to_id.get(key) {
            return id.clone();
        }

        let id = NoteId::new();
        self.key_to_id.insert(key.clone(), id.clone());
        self.id_to_key.insert(id.clone(), key.clone());
        id
    }

    #[allow(private_interfaces)]
    pub fn rebind(&mut self, old: &NoteKey, new: &NoteKey) -> Option<NoteId> {
        let id = self.key_to_id.remove(old)?;
        self.key_to_id.insert(new.clone(), id.clone());
        self.id_to_key.insert(id.clone(), new.clone());
        Some(id)
    }

    #[allow(private_interfaces)]
    pub fn lookup(&self, key: &NoteKey) -> Option<NoteId> {
        self.key_to_id.get(key).cloned()
    }

    #[allow(private_interfaces)]
    pub fn key_of(&self, id: &NoteId) -> Option<NoteKey> {
        self.id_to_key.get(id).cloned()
    }
}

impl Default for IdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
