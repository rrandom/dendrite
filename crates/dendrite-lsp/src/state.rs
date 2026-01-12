use dendrite_core::refactor::model::EditPlan;
use dendrite_core::workspace::vfs::FileSystem;
use dendrite_core::workspace::Vault;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

/// Global state for LSP server
/// Must be Send + Sync
#[derive(Clone)]
pub struct GlobalState {
    /// RwLock-protected Core Vault (Workspace + FS)
    pub vault: Arc<RwLock<Option<Vault>>>,
    /// Document content cache (URI -> content)
    /// Stores the current document text for completion and other operations
    pub document_cache: Arc<RwLock<HashMap<Url, String>>>,
    /// Virtual File System backend
    pub fs: Arc<dyn FileSystem>,
    /// History of applied refactors for multi-level undo
    pub refactor_history: Arc<RwLock<VecDeque<EditPlan>>>,
}

impl GlobalState {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            vault: Arc::new(RwLock::new(None)),
            document_cache: Arc::new(RwLock::new(HashMap::new())),
            fs,
            refactor_history: Arc::new(RwLock::new(VecDeque::with_capacity(5))),
        }
    }
}
