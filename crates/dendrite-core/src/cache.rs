use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::identity::IdentityRegistry;
use crate::store::Store;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMetadata {
    pub mtime: SystemTime,
    pub size: u64,
    pub digest: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistentState {
    pub version: u32,
    pub model_id: String,
    pub store: Store,
    pub identity: IdentityRegistry,
    pub metadata: HashMap<PathBuf, FileMetadata>,
}

impl PersistentState {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new(model_id: String, store: Store, identity: IdentityRegistry) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            model_id,
            store,
            identity,
            metadata: HashMap::new(),
        }
    }

    pub fn load(
        path: &Path,
        fs: &dyn crate::vfs::FileSystem,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let buffer = fs.read_all(path)?;
        let state: PersistentState = bincode::deserialize(&buffer)?;

        if state.version != Self::CURRENT_VERSION {
            return Err("Incompatible cache version".into());
        }

        Ok(state)
    }

    pub fn save(
        &self,
        path: &Path,
        fs: &dyn crate::vfs::FileSystem,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let buffer = bincode::serialize(self)?;
        fs.write_all(path, &buffer)?;
        Ok(())
    }
}
