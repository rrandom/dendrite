use crate::{
    model::{NoteId, NoteKey, ResolverId},
    normalize_path_to_id,
};
pub trait IdentityRegistry: Send + Sync {
    fn get_or_create(&mut self, resolver: ResolverId, key: &NoteKey) -> NoteId;
    fn lookup(&self, resolver: ResolverId, key: &NoteKey) -> Option<NoteId>;
    fn key_of(&self, id: &NoteId) -> Option<(ResolverId, NoteKey)>;
}

pub struct DendriteIdentityRegistry;

impl IdentityRegistry for DendriteIdentityRegistry {
    fn get_or_create(&mut self, resolver: ResolverId, key: &NoteKey) -> NoteId {
        todo!();
    }

    fn lookup(&self, resolver: ResolverId, key: &NoteKey) -> Option<NoteId> {
        todo!()
    }

    fn key_of(&self, id: &NoteId) -> Option<(ResolverId, NoteKey)> {
        todo!()
    }
}
