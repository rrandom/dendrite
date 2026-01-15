use std::sync::RwLock;

use crate::identity::IdentityRegistry;
use crate::semantic::SemanticModel;
use crate::store::Store;

mod assembler;
mod indexer;
mod note_tree;
mod queries;
mod refactor_api;
mod sync_ops;
mod vault;

#[cfg(test)]
mod tests;

pub use crate::vfs::FileSystem;
pub use indexer::Indexer;
use note_tree::NoteTree;
pub use vault::Vault;

pub struct Workspace {
    pub(crate) model: Box<dyn SemanticModel>,
    pub(crate) identity: IdentityRegistry,
    pub(crate) store: Store,
    pub(crate) tree_cache: RwLock<Option<NoteTree>>,
}

impl Workspace {
    pub fn new(model: Box<dyn SemanticModel>) -> Self {
        Self {
            model,
            identity: IdentityRegistry::new(),
            store: Store::new(),
            tree_cache: RwLock::new(None),
        }
    }
}
