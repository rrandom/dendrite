//! Dendrite LSP Library
//!
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService};
use std::path::PathBuf;
use url::Url;

use crate::state::GlobalState;

mod handlers;
mod state;
mod conversion;
mod protocol;
/// LSP backend implementation
pub struct Backend {
    client: Client,
    state: GlobalState,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: GlobalState::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        let root_uri = params.root_uri;

        if let Some(uri) = root_uri {
            if let Ok(root_path) = uri.to_file_path() {
                self.client.log_message(MessageType::INFO, format!("Initializing workspace at: {:?}", root_path)).await;

                let mut ws = dendrite_core::workspace::Workspace::new(root_path);
                let files = ws.scan();
                
                self.client.log_message(MessageType::INFO, format!("Found {} markdown files:", files.len())).await;
                for file in &files {
                    self.client.log_message(MessageType::INFO, format!(" - {:?}", file)).await;
                }

                let mut workspace = self.state.workspace.write().await;
                *workspace = Some(ws);
            }
        } else {
            self.client.log_message(MessageType::WARNING, "No rootUri provided!").await;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        eprintln!("✅ Client initialized, ready to accept requests");
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        eprintln!("🛑 Shutdown requested");
        Ok(())
    }
}

// Utility functions for URI/Path conversion (for future use)
#[allow(dead_code)]
fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok()
}

#[allow(dead_code)]
fn path_to_uri(path: &PathBuf) -> Option<Url> {
    Url::from_file_path(path).ok()
}

/// Create and return LSP service and client socket
pub fn create_lsp_service() -> (LspService<Backend>, tower_lsp::ClientSocket) {
    LspService::new(|client| Backend::new(client))
}
