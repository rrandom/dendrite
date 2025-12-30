use std::sync::Arc;
use tokio::sync::RwLock;
use dendrite_core::Workspace;

/// Global state for LSP server
/// Must be Send + Sync
#[derive(Clone)]
pub struct GlobalState {
    /// RwLock-protected Core Workspace
    /// Read operations (completion, goto) are concurrent
    /// Write operations (didChange) are exclusive
    pub workspace: Arc<RwLock<Option<Workspace>>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            workspace: Arc::new(RwLock::new(None)),
        }
    }
}