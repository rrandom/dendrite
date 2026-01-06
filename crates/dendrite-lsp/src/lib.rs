//! Dendrite LSP Library
//!
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService};

use crate::protocol::{GetHierarchyParams, ListNotesParams};
use state::GlobalState;

mod conversion;
mod handlers;
mod protocol;
mod state;

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
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        handlers::handle_initialize(&self.client, &self.state, params).await
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        eprintln!("✅ Client initialized, ready to accept requests");
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        eprintln!("🛑 Shutdown requested");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        handlers::handle_did_open(&self.state, params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        handlers::handle_did_change(&self.state, params).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        handlers::handle_did_change_watched_files(&self.state, params).await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        handlers::handle_goto_definition(&self.client, &self.state, params).await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        handlers::handle_completion(&self.client, &self.state, params).await
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        handlers::handle_hover(&self.client, &self.state, params).await
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        handlers::handle_document_highlight(&self.client, &self.state, params).await
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> tower_lsp::jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "dendrite/getHierarchy" => {
                let params = GetHierarchyParams::default();
                let result =
                    handlers::handle_get_hierarchy(&self.client, &self.state, params).await?;
                serde_json::to_value(result).map(Some).map_err(|e| Error {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to serialize result: {}", e).into(),
                    data: None,
                })
            }
            "dendrite/listNotes" => {
                let list_params = if let Some(first_arg) = params.arguments.first() {
                    serde_json::from_value::<ListNotesParams>(first_arg.clone()).unwrap_or_default()
                } else {
                    ListNotesParams::default()
                };
                let result =
                    handlers::handle_list_notes(&self.client, &self.state, list_params).await?;
                serde_json::to_value(result).map(Some).map_err(|e| Error {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to serialize result: {}", e).into(),
                    data: None,
                })
            }
            _ => Err(Error {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown command: {}", params.command).into(),
                data: None,
            }),
        }
    }
}

/// Create and return LSP service and client socket
pub fn create_lsp_service() -> (LspService<Backend>, tower_lsp::ClientSocket) {
    LspService::new(|client| Backend::new(client))
}
