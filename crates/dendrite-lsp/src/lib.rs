//! Dendrite LSP Library
//!
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService};

use dendrite_core::{DendriteIdentityRegistry, DendronStrategy, Workspace};
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
        let root_uri = params.root_uri;

        if let Some(uri) = root_uri {
            if let Ok(root_path) = uri.to_file_path() {
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("Initializing workspace at: {:?}", root_path),
                    )
                    .await;

                let root_path_clone = root_path.clone();
                let (ws, files) = tokio::task::spawn_blocking(move || {
                    let mut workspace = Workspace::new(
                        root_path_clone,
                        Box::new(DendronStrategy::new()),
                        Box::new(DendriteIdentityRegistry::new()),
                    );
                    let files = workspace.initialize();
                    (workspace, files)
                })
                .await
                .map_err(|e| tower_lsp::jsonrpc::Error {
                    code: tower_lsp::jsonrpc::ErrorCode::InternalError,
                    message: format!("Failed to initialize workspace: {}", e).into(),
                    data: None,
                })?;

                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("Found {} markdown files:", files.len()),
                    )
                    .await;
                for file in &files {
                    self.client
                        .log_message(MessageType::INFO, format!(" - {:?}", file))
                        .await;
                }

                let notes_count = ws.all_notes().len();
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("Parsed {} notes from workspace", notes_count),
                    )
                    .await;

                for note in ws.all_notes() {
                    self.client
                        .log_message(
                            MessageType::INFO,
                            format!("Note: (title: {:?})", note.title),
                        )
                        .await;

                    if !note.headings.is_empty() {
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("  Headings ({}):", note.headings.len()),
                            )
                            .await;
                        for heading in &note.headings {
                            self.client
                                .log_message(
                                    MessageType::INFO,
                                    format!("    H{}: {}", heading.level, heading.text),
                                )
                                .await;
                        }
                    }

                    if !note.links.is_empty() {
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("  Links ({}):", note.links.len()),
                            )
                            .await;
                        for link in &note.links {
                            self.client
                                .log_message(
                                    MessageType::INFO,
                                    format!("    -> Link (kind: {:?})", link.kind),
                                )
                                .await;
                        }
                    }

                    if note.frontmatter.is_some() {
                        self.client
                            .log_message(MessageType::INFO, "  Has frontmatter")
                            .await;
                    }
                }

                let mut workspace = self.state.workspace.write().await;
                *workspace = Some(ws);
            }
        } else {
            self.client
                .log_message(MessageType::WARNING, "No rootUri provided!")
                .await;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut state = self.state.workspace.write().await;
        if let Some(ws) = &mut *state {
            if let Ok(path) = params.text_document.uri.to_file_path() {
                let text = params.text_document.text;
                ws.on_file_open(path, text);
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut state = self.state.workspace.write().await;
        if let Some(ws) = &mut *state {
            if let Ok(path) = params.text_document.uri.to_file_path() {
                // With FULL sync, the last change contains the full document text
                if let Some(last_change) = params.content_changes.last() {
                    ws.on_file_changed(path, last_change.text.clone());
                }
            }
        }
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let mut state = self.state.workspace.write().await;
        if let Some(ws) = &mut *state {
            for change in params.changes {
                if let Ok(path) = change.uri.to_file_path() {
                    match change.typ {
                        FileChangeType::CREATED => {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                ws.update_file(&path, &content);
                            }
                        }
                        FileChangeType::CHANGED => {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                ws.update_file(&path, &content);
                            }
                        }
                        FileChangeType::DELETED => {
                            ws.on_file_delete(path);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Create and return LSP service and client socket
pub fn create_lsp_service() -> (LspService<Backend>, tower_lsp::ClientSocket) {
    LspService::new(|client| Backend::new(client))
}
