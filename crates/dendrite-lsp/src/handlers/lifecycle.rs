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
            for file in &files {
                client
                    .log_message(MessageType::INFO, format!(" - {:?}", file))
                    .await;
            }

            let notes_count = vault.workspace.all_notes().len();
            client
                .log_message(
                    MessageType::INFO,
                    format!("Parsed {} notes from workspace", notes_count),
                )
                .await;

            for note in vault.workspace.all_notes() {
                client
                    .log_message(
                        MessageType::INFO,
                        format!("Note: (title: {:?})", note.title),
                    )
                    .await;

                if !note.headings.is_empty() {
                    client
                        .log_message(
                            MessageType::INFO,
                            format!("  Headings ({}):", note.headings.len()),
                        )
                        .await;
                    for heading in &note.headings {
                        client
                            .log_message(
                                MessageType::INFO,
                                format!("    H{}: {}", heading.level, heading.text),
                            )
                            .await;
                    }
                }

                if !note.links.is_empty() {
                    client
                        .log_message(
                            MessageType::INFO,
                            format!("  Links ({}):", note.links.len()),
                        )
                        .await;
                    for link in &note.links {
                        client
                            .log_message(
                                MessageType::INFO,
                                format!("    -> Link (kind: {:?})", link.kind),
                            )
                            .await;
                    }
                }

                if note.frontmatter.is_some() {
                    client
                        .log_message(MessageType::INFO, "  Has frontmatter")
                        .await;
                }
            }

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
