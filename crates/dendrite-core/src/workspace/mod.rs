use std::path::PathBuf;
use std::sync::RwLock;

use crate::hierarchy::SyntaxStrategy;
use crate::identity::IdentityRegistry;
use crate::store::Store;
pub mod vfs;

mod assembler;
mod hierarchy;
mod indexer;
mod queries;
mod vault;

#[cfg(test)]
mod tests;

use hierarchy::NoteTree;
pub use indexer::Indexer;
pub use vault::Vault;
pub use vfs::FileSystem;

pub struct Workspace {
    pub(crate) resolver: Box<dyn SyntaxStrategy>,
    pub(crate) identity: IdentityRegistry,
    pub(crate) store: Store,
    pub(crate) tree_cache: RwLock<Option<NoteTree>>,
}

impl Workspace {
    pub fn new(resolver: Box<dyn SyntaxStrategy>, identity: IdentityRegistry) -> Self {
        Self {
            resolver,
            identity,
            store: Store::new(),
            tree_cache: RwLock::new(None),
        }
    }

    pub fn initialize(&mut self, root: PathBuf, fs: &dyn FileSystem) -> Vec<PathBuf> {
        let mut indexer = Indexer::new(self, fs);
        indexer.full_index(root)
    }
}
