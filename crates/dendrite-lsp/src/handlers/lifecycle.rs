use crate::state::GlobalState;
use dendrite_core::{DendronModel, Vault, Workspace};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

/// Handle "initialize" request
pub async fn handle_initialize(
    client: &Client,
    state: &GlobalState,
    params: InitializeParams,
) -> Result<InitializeResult> {
    let root_uri = params.root_uri;

    if let Some(uri) = root_uri {
        if let Ok(root_path) = uri.to_file_path() {
            client
                .log_message(
                    MessageType::INFO,
                    format!("Initializing workspace at: {:?}", root_path),
                )
                .await;

            let root_path_clone = root_path.clone();
            let fs = state.fs.clone();
            let (vault, files) = tokio::task::spawn_blocking(move || {
                let workspace =
                    Workspace::new(Box::new(DendronModel::new(root_path_clone.clone())));
                let mut v = Vault::new(workspace, fs);
                let files = v.initialize(root_path_clone);
                (v, files)
            })
            .await
            .map_err(|e| tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InternalError,
                message: format!("Failed to initialize workspace: {}", e).into(),
                data: None,
            })?;

            client
                .log_message(MessageType::INFO, format!("Found {} files:", files.len()))
                .await;

            let notes_count = vault.workspace.all_notes().len();
            client
                .log_message(
                    MessageType::INFO,
                    format!("✅ Parsed {} notes from workspace", notes_count),
                )
                .await;

            let mut vault_lock = state.vault.write().await;
            *vault_lock = Some(vault);
        }
    } else {
        client
            .log_message(MessageType::WARNING, "No rootUri provided!")
            .await;
    }

    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    will_save: Some(false),
                    will_save_wait_until: Some(true),
                    save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                }
                .into(),
            ),
            definition_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Left(true)),
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
                    "dendrite/getNoteKey".to_string(),
                    "dendrite/undoMutation".to_string(),
                    "dendrite/splitNote".to_string(),
                    "dendrite/reorganizeHierarchy".to_string(),
                    "dendrite/workspaceAudit".to_string(),
                    "dendrite/deleteNote".to_string(),
                    "dendrite/getBacklinks".to_string(),
                ],
                work_done_progress_options: Default::default(),
            }),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    legend: super::get_legend(),
                    range: Some(false),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                }),
            ),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: None,
                file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                    did_rename: Some(FileOperationRegistrationOptions {
                        filters: vec![FileOperationFilter {
                            scheme: Some("file".to_string()),
                            pattern: FileOperationPattern {
                                glob: "**/*.md".to_string(),
                                matches: None,
                                options: None,
                            },
                        }],
                    }),
                    ..Default::default()
                }),
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}
