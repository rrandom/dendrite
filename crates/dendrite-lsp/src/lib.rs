//! Dendrite LSP Library
//! 
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use tower_lsp::LspService;

/// LSP backend implementation
pub struct Backend {
    // TODO: Add necessary fields
}

impl Backend {
    pub fn new() -> Self {
        Self {}
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for Backend {
    async fn initialize(&self, params: tower_lsp::lsp_types::InitializeParams) -> tower_lsp::jsonrpc::Result<tower_lsp::lsp_types::InitializeResult> {
        // Parse rootUri from client
        if let Some(ref root_uri) = params.root_uri {
            eprintln!("📁 Workspace root: {}", root_uri);
        } else {
            eprintln!("⚠️  No workspace root provided");
        }

        // Return capabilities (empty for now, as per Week 3 goal)
        let capabilities = tower_lsp::lsp_types::ServerCapabilities::default();
        
        Ok(tower_lsp::lsp_types::InitializeResult {
            capabilities,
            server_info: Some(tower_lsp::lsp_types::ServerInfo {
                name: "Dendrite".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
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

/// Create and return LSP service and client socket
pub fn create_lsp_service() -> (LspService<Backend>, tower_lsp::ClientSocket) {
    LspService::new(|_client| Backend::new())
}
