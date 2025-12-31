use crate::model::{NoteId, NoteKey, ResolverId};
use std::collections::HashMap;

/// NoteKey <=> NoteId
#[allow(private_interfaces)]
pub trait IdentityRegistry: Send + Sync {
    fn get_or_create(&mut self, key: &NoteKey) -> NoteId;
    fn lookup(&self, key: &NoteKey) -> Option<NoteId>;
    fn rebind(&mut self, old: &NoteKey, new: &NoteKey) -> Option<NoteId>;
    fn key_of(&self, id: &NoteId) -> Option<(ResolverId, NoteKey)>;
}

pub struct DendriteIdentityRegistry {
    key_to_id: HashMap<NoteKey, NoteId>,
    id_to_key: HashMap<NoteId, NoteKey>,
}

impl DendriteIdentityRegistry {
    pub fn new() -> Self {
        Self {
            key_to_id: HashMap::new(),
            id_to_key: HashMap::new(),
        }
    }
}

#[allow(private_interfaces)]
impl IdentityRegistry for DendriteIdentityRegistry {
    fn get_or_create(&mut self, key: &NoteKey) -> NoteId {
        if let Some(id) = self.key_to_id.get(key) {
            return id.clone();
        }

        let id = NoteId::new();
        self.key_to_id.insert(key.clone(), id.clone());
        self.id_to_key.insert(id.clone(), key.clone());
        id
    }

    fn rebind(&mut self, old: &NoteKey, new: &NoteKey) -> Option<NoteId> {
        let id = self.key_to_id.remove(old)?;
        self.key_to_id.insert(new.clone(), id.clone());
        self.id_to_key.insert(id.clone(), new.clone());
        Some(id)
    }

    fn lookup(&self, key: &NoteKey) -> Option<NoteId> {
        self.key_to_id.get(key).cloned()
    }

    fn key_of(&self, id: &NoteId) -> Option<(ResolverId, NoteKey)> {
        self.id_to_key
            .get(id)
            .cloned()
            .map(|key| (ResolverId("Dendrite"), key))
    }
}
