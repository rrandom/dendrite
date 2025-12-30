//! Dendrite LSP Library
//! 
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use std::path::PathBuf;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService};
use url::Url;

use crate::state::GlobalState;

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

                let mut ws = dendrite_core::workspace::Workspace::new(root_path, Box::new(dendrite_core::hierarchy::dendron::DendronStrategy), Box::new(dendrite_core::identity::DendriteIdentityRegistry));
                let files = ws.initialize();

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

                // Display each note's parsed content
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
                                    format!("    -> {:?}", 11),
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

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let mut state = self.state.workspace.write().await;
        if let Some(ws) = &mut *state {
            for change in params.changes {
                // 将 URI 转为绝对路径
                if let Ok(path) = change.uri.to_file_path() {
                    match change.typ {
                        // 创建 (Created)
                        FileChangeType::CREATED => {
                            // 读取磁盘内容
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                ws.update_file(&path, &content);
                            }
                        }
                        // 修改 (Changed)
                        FileChangeType::CHANGED => {
                            // 读取磁盘内容
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                ws.update_file(&path, &content);
                            }
                        }
                        // 删除 (Deleted)
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
