//! Dendrite LSP Library
//!
//! LSP protocol layer, converts JSON-RPC requests to Core library calls.

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService};

use conversion::{lsp_position_to_point, path_to_uri, text_range_to_lsp_range};
use dendrite_core::{DendriteIdentityRegistry, DendronStrategy, Workspace};
use handlers::{handle_get_hierarchy, handle_list_notes};
use protocol::{GetHierarchyParams, ListNotesParams};
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
                        root_path_clone.clone(),
                        Box::new(DendronStrategy::new(root_path_clone.clone())),
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
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["[".to_string()]),
                    all_commit_characters: None,
                    resolve_provider: Some(false),
                    work_done_progress_options: Default::default(),
                    completion_item: Default::default(),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "dendrite/getHierarchy".to_string(),
                        "dendrite/listNotes".to_string(),
                    ],
                    work_done_progress_options: Default::default(),
                }),
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
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();

        // Update document cache
        {
            let mut cache = self.state.document_cache.write().await;
            cache.insert(uri.clone(), text.clone());
        }

        // Update workspace
        let mut state = self.state.workspace.write().await;
        if let Some(ws) = &mut *state {
            if let Ok(path) = uri.to_file_path() {
                ws.on_file_open(path, text);
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        // With FULL sync, the last change contains the full document text
        if let Some(last_change) = params.content_changes.last() {
            let text = last_change.text.clone();

            // Update document cache
            {
                let mut cache = self.state.document_cache.write().await;
                cache.insert(uri.clone(), text.clone());
            }

            // Update workspace
            let mut state = self.state.workspace.write().await;
            if let Some(ws) = &mut *state {
                if let Ok(path) = uri.to_file_path() {
                    ws.on_file_changed(path, text);
                }
            }
        }
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let mut state = self.state.workspace.write().await;
        if let Some(ws) = &mut *state {
            for change in params.changes {
                let uri = change.uri.clone();
                if let Ok(path) = uri.to_file_path() {
                    match change.typ {
                        FileChangeType::CREATED => {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                // Update cache
                                {
                                    let mut cache = self.state.document_cache.write().await;
                                    cache.insert(uri, content.clone());
                                }
                                ws.update_file(&path, &content);
                            }
                        }
                        FileChangeType::CHANGED => {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                // Update cache
                                {
                                    let mut cache = self.state.document_cache.write().await;
                                    cache.insert(uri, content.clone());
                                }
                                ws.update_file(&path, &content);
                            }
                        }
                        FileChangeType::DELETED => {
                            // Remove from cache
                            {
                                let mut cache = self.state.document_cache.write().await;
                                cache.remove(&uri);
                            }
                            ws.on_file_delete(path);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let state = self.state.workspace.read().await;
        let Some(ws) = &*state else {
            return Ok(None);
        };

        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Ok(path) = uri.to_file_path() else {
            return Ok(None);
        };

        // Convert LSP Position to Core Point
        let point = lsp_position_to_point(position);

        // Find the link at the given position
        let Some(link) = ws.find_link_at_position(&path, point) else {
            return Ok(None);
        };

        // Get the target note's path
        let Some(target_path) = ws.get_link_target_path(link) else {
            return Ok(None);
        };

        // Convert path to URI
        let Some(target_uri) = path_to_uri(&target_path) else {
            return Ok(None);
        };

        // Return the definition location
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
        })))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "🔍 Completion requested at {:?}",
                    params.text_document_position.position
                ),
            )
            .await;

        let state = self.state.workspace.read().await;
        let Some(ws) = &*state else {
            self.client
                .log_message(MessageType::WARNING, "⚠️ Workspace not initialized")
                .await;
            return Ok(None);
        };

        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📄 URI: {:?}, Position: line {}, char {}",
                    uri, position.line, position.character
                ),
            )
            .await;

        // Get document content from cache (which is updated by did_open/did_change)
        let document_text = {
            let cache = self.state.document_cache.read().await;
            cache.get(uri).cloned()
        };

        let Some(document_text) = document_text else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("❌ Document not found in cache: {:?}", uri),
                )
                .await;
            return Ok(None);
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📄 Document text length: {} characters",
                    document_text.len()
                ),
            )
            .await;

        // Check if we're in a [[ context (not just [ or [[[)
        let line_idx = position.line as usize;
        let char_idx = position.character as usize;

        self.client
            .log_message(
                MessageType::INFO,
                format!("📝 Line index: {}, Char index: {}", line_idx, char_idx),
            )
            .await;

        let lines: Vec<&str> = document_text.lines().collect();
        if line_idx >= lines.len() {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!(
                        "❌ Line index {} out of range (total lines: {})",
                        line_idx,
                        lines.len()
                    ),
                )
                .await;
            return Ok(None);
        }

        let current_line = lines[line_idx];

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📄 Current line (len={}): {:?}",
                    current_line.len(),
                    current_line
                ),
            )
            .await;

        // For simplicity, we'll work with character indices
        // Since '[' is ASCII, char_idx should work correctly
        // Note: LSP Position uses UTF-16 character indices, but for ASCII characters
        // like '[', UTF-16 and UTF-8 indices are the same
        if char_idx < 2 {
            // Not enough characters before cursor for [[
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "❌ Not enough characters before cursor (need 2, have {})",
                        char_idx
                    ),
                )
                .await;
            return Ok(None);
        }

        // Get characters before cursor position
        // We need to check if we have exactly [[ (not [ or [[[)
        let chars_before: Vec<char> = current_line
            .chars()
            .take(char_idx)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(3)
            .collect();

        self.client
            .log_message(
                MessageType::INFO,
                format!("🔤 Characters before cursor (reversed): {:?}", chars_before),
            )
            .await;

        if chars_before.len() < 2 {
            self.client
                .log_message(MessageType::INFO, "❌ Not enough characters collected")
                .await;
            return Ok(None);
        }

        // Check if the two characters immediately before cursor are both '['
        // chars_before[0] is the character right before cursor, [1] is before that
        let is_double_bracket = chars_before[0] == '[' && chars_before[1] == '[';

        self.client
            .log_message(
                MessageType::INFO,
                format!("✅ Is double bracket [[: {}", is_double_bracket),
            )
            .await;

        if !is_double_bracket {
            // Not in [[ context (might be just [ or something else)
            self.client
                .log_message(MessageType::INFO, "❌ Not in [[ context")
                .await;
            return Ok(None);
        }

        // Check if there's a third '[' before (which would make it [[[)
        if chars_before.len() >= 3 && chars_before[2] == '[' {
            // We have [[[ or more, don't trigger
            self.client
                .log_message(MessageType::INFO, "❌ Has [[[ or more, not triggering")
                .await;
            return Ok(None);
        }

        self.client
            .log_message(
                MessageType::INFO,
                "✅ Context check passed, providing completions",
            )
            .await;

        // Get all note keys for completion
        let note_keys = ws.all_note_keys();

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📋 Found {} notes for completion: {}",
                    note_keys.len(),
                    note_keys
                        .iter()
                        .map(|(k, d)| format!("{}: {}", k, d))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
            .await;

        // Create completion items
        let items: Vec<CompletionItem> = note_keys
            .into_iter()
            .map(|(key, display_name)| {
                // Use note key as label (as requested)
                // Display name can be shown in detail if available
                let detail = if display_name.is_empty() {
                    None
                } else {
                    Some(display_name)
                };

                CompletionItem {
                    label: key.clone(),
                    kind: Some(CompletionItemKind::FILE),
                    detail,
                    // Insert the key when user selects
                    insert_text: Some(key),
                    ..Default::default()
                }
            })
            .collect();

        self.client
            .log_message(
                MessageType::INFO,
                format!("✅ Returning {} completion items", items.len()),
            )
            .await;

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "🖱️ Hover requested at {:?}",
                    params.text_document_position_params.position
                ),
            )
            .await;

        let state = self.state.workspace.read().await;
        let Some(ws) = &*state else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    "⚠️ Workspace not initialized for hover",
                )
                .await;
            return Ok(None);
        };

        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📄 Hover URI: {:?}, Position: line {}, char {}",
                    uri, position.line, position.character
                ),
            )
            .await;

        let Ok(path) = uri.to_file_path() else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    "❌ Failed to convert URI to file path for hover",
                )
                .await;
            return Ok(None);
        };

        // Convert LSP Position to Core Point
        let point = lsp_position_to_point(position);

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📍 Converted to Point: line {}, col {}",
                    point.line, point.col
                ),
            )
            .await;

        // Find the link at the given position
        let Some(link) = ws.find_link_at_position(&path, point) else {
            self.client
                .log_message(MessageType::INFO, "❌ No link found at position")
                .await;
            return Ok(None);
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "🔗 Found link at range: line {}:{}-{}:{}",
                    link.range.start.line,
                    link.range.start.col,
                    link.range.end.line,
                    link.range.end.col
                ),
            )
            .await;

        // Get the target note's path for hover information
        let target_path = ws.get_link_target_path(link);
        let target_info = if let Some(path) = &target_path {
            format!("Target: {:?}", path)
        } else {
            "Target: (not found)".to_string()
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!("🎯 Target path: {:?}", target_path),
            )
            .await;

        // Convert link range to LSP range for hover highlighting
        let link_range = text_range_to_lsp_range(link.range);

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "📏 Link range (LSP): {:?} - {:?}",
                    link_range.start, link_range.end
                ),
            )
            .await;

        // Return hover with the link range for proper highlighting
        let hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: target_info,
            }),
            range: Some(link_range),
        };

        self.client
            .log_message(MessageType::INFO, "✅ Returning hover response")
            .await;

        Ok(Some(hover))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "✨ Document highlight requested at {:?}",
                    params.text_document_position_params.position
                ),
            )
            .await;

        let state = self.state.workspace.read().await;
        let Some(ws) = &*state else {
            return Ok(None);
        };

        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Ok(path) = uri.to_file_path() else {
            return Ok(None);
        };

        // Convert LSP Position to Core Point
        let point = lsp_position_to_point(position);

        // Find the link at the given position
        let Some(link) = ws.find_link_at_position(&path, point) else {
            self.client
                .log_message(MessageType::INFO, "❌ No link found for highlight")
                .await;
            return Ok(None);
        };

        // Convert link range to LSP range for highlighting
        let link_range = text_range_to_lsp_range(link.range);

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "✨ Highlighting link range: {:?} - {:?}",
                    link_range.start, link_range.end
                ),
            )
            .await;

        // Return the highlight
        Ok(Some(vec![DocumentHighlight {
            range: link_range,
            kind: Some(DocumentHighlightKind::TEXT),
        }]))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> tower_lsp::jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "dendrite/getHierarchy" => {
                let params = GetHierarchyParams::default();
                let result = handle_get_hierarchy(&self.client, &self.state, params).await?;
                serde_json::to_value(result).map(Some).map_err(|e| Error {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to serialize result: {}", e).into(),
                    data: None,
                })
            }
            "dendrite/listNotes" => {
                // Parse params from arguments (if provided)
                let list_params = if let Some(first_arg) = params.arguments.first() {
                    serde_json::from_value::<ListNotesParams>(first_arg.clone()).unwrap_or_default()
                } else {
                    ListNotesParams::default()
                };
                let result = handle_list_notes(&self.client, &self.state, list_params).await?;
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
