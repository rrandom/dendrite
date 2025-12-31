use dendrite_core::Workspace;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

/// Global state for LSP server
/// Must be Send + Sync
#[derive(Clone)]
pub struct GlobalState {
    /// RwLock-protected Core Workspace
    /// Read operations (completion, goto) are concurrent
    /// Write operations (didChange) are exclusive
    pub workspace: Arc<RwLock<Option<Workspace>>>,
    /// Document content cache (URI -> content)
    /// Stores the current document text for completion and other operations
    pub document_cache: Arc<RwLock<HashMap<Url, String>>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            workspace: Arc::new(RwLock::new(None)),
            document_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
