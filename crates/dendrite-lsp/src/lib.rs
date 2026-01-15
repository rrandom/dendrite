//! Dendrite LSP Library
//!
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService};

use crate::protocol::{GetHierarchyParams, ListNotesParams};
use dendrite_core::vfs::PhysicalFileSystem;
use state::GlobalState;
use std::sync::Arc;

mod conversion;
mod handlers;
mod protocol;
mod state;
#[cfg(test)]
mod tests;

/// LSP backend implementation
#[derive(Clone)]
pub struct Backend {
    pub(crate) client: Client,
    pub(crate) state: GlobalState,
}

impl Backend {
    pub fn new(client: Client, fs: Arc<PhysicalFileSystem>) -> Self {
        Self {
            client,
            state: GlobalState::new(fs),
        }
    }

    pub async fn handle_execute_command(
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
            "dendrite/getNoteKey" => {
                let params = if let Some(first_arg) = params.arguments.first() {
                    serde_json::from_value::<crate::protocol::GetNoteKeyParams>(first_arg.clone())
                        .map_err(|e| Error {
                        code: ErrorCode::InvalidParams,
                        message: format!("Invalid parameters: {}", e).into(),
                        data: None,
                    })?
                } else {
                    return Err(Error {
                        code: ErrorCode::InvalidParams,
                        message: "Missing parameters".into(),
                        data: None,
                    });
                };
                let result =
                    handlers::handle_get_note_key(&self.client, &self.state, params).await?;
                serde_json::to_value(result).map(Some).map_err(|e| Error {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to serialize result: {}", e).into(),
                    data: None,
                })
            }
            "dendrite/undoRefactor" => {
                handlers::handle_undo_refactor(&self.client, &self.state).await?;
                Ok(None)
            }
            "dendrite/splitNote" => {
                handlers::handle_split_note_command(&self.client, &self.state, params).await
            }
            "dendrite/reorganizeHierarchy" => {
                handlers::hierarchy::handle_reorganize_hierarchy_command(
                    &self.client,
                    &self.state,
                    params,
                )
                .await
            }
            "dendrite/workspaceAudit" => {
                handlers::handle_workspace_audit_command(&self.client, &self.state, params).await
            }
            "dendrite/resolveHierarchyEdits" => {
                handlers::hierarchy::handle_resolve_hierarchy_edits(
                    &self.client,
                    &self.state,
                    params,
                )
                .await
            }
            _ => Err(Error {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown command: {}", params.command).into(),
                data: None,
            }),
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

    async fn did_rename_files(&self, params: RenameFilesParams) {
        handlers::handle_did_rename_files(&self.client, &self.state, params).await;
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

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SemanticTokensResult>> {
        handlers::handle_semantic_tokens_full(&self.client, &self.state, params).await
    }

    async fn rename(
        &self,
        params: RenameParams,
    ) -> tower_lsp::jsonrpc::Result<Option<WorkspaceEdit>> {
        handlers::rename::handle_rename(&self.client, &self.state, params).await
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        handlers::handle_code_action(&self.client, &self.state, params).await
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> tower_lsp::jsonrpc::Result<Option<serde_json::Value>> {
        self.handle_execute_command(params).await
    }
}

/// Create and return LSP service and client socket
pub fn create_lsp_service() -> (LspService<Backend>, tower_lsp::ClientSocket) {
    let fs = Arc::new(PhysicalFileSystem);
    LspService::new(|client| Backend::new(client, fs))
}
