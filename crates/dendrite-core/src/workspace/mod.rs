use crate::cache::FileMetadata;
use crate::identity::IdentityRegistry;
use crate::semantic::SemanticModel;
use crate::store::Store;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

mod assembler;
mod engine;
mod indexer;
mod mutations;
mod note_tree;
mod queries;
mod sync_ops;

#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod multi_vault_tests;
#[cfg(test)]
mod tests;

pub use crate::vfs::FileSystem;
pub use engine::DendriteEngine;
pub use indexer::Indexer;
use note_tree::NoteTree;

pub struct Workspace {
    pub(crate) config: crate::config::DendriteConfig,
    pub(crate) model: Box<dyn SemanticModel>,
    pub(crate) identity: IdentityRegistry,
    pub(crate) store: Store,
    pub(crate) tree_cache: RwLock<Option<NoteTree>>,
    pub(crate) cache_metadata: HashMap<PathBuf, FileMetadata>,
}

impl Workspace {
    pub fn new(config: crate::config::DendriteConfig, model: Box<dyn SemanticModel>) -> Self {
        Self {
            config,
            model,
            identity: IdentityRegistry::new(),
            store: Store::new(),
            tree_cache: RwLock::new(None),
            cache_metadata: HashMap::new(),
        }
    }

    pub fn vault_name_for_path(&self, path: &std::path::Path) -> Option<String> {
        for vault in &self.config.workspace.vaults {
            if path.starts_with(&vault.path) {
                return Some(vault.name.clone());
            }
        }
        None
    }
}
