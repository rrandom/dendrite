use crate::state::GlobalState;
use dendrite_core::{DendronStrategy, Vault, Workspace};
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
                    Workspace::new(Box::new(DendronStrategy::new(root_path_clone.clone())));
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
                .log_message(
                    MessageType::INFO,
                    format!("Found {} markdown files:", files.len()),
                )
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
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
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
                ],
                work_done_progress_options: Default::default(),
            }),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    legend: super::get_legend(),
                    range: Some(false),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                }),
            ),
            ..Default::default()
        },
        ..Default::default()
    })
}
