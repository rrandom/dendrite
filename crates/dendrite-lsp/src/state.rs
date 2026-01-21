use dendrite_core::mutation::model::EditPlan;
use dendrite_core::vfs::FileSystem;
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
    /// History of applied mutations for multi-level undo
    pub mutation_history: Arc<RwLock<VecDeque<EditPlan>>>,
    /// Signal to trigger debounced cache saving
    pub(crate) dirty_signal: tokio::sync::mpsc::UnboundedSender<()>,
}

impl GlobalState {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        let (dirty_tx, dirty_rx) = tokio::sync::mpsc::unbounded_channel();

        let state = Self {
            vault: Arc::new(RwLock::new(None)),
            document_cache: Arc::new(RwLock::new(HashMap::new())),
            fs,
            mutation_history: Arc::new(RwLock::new(VecDeque::with_capacity(5))),
            dirty_signal: dirty_tx,
        };

        // Start background cache manager
        let manager = crate::cache_manager::CacheManager::new(state.clone(), dirty_rx);
        tokio::spawn(async move {
            manager.start().await;
        });

        state
    }
}
