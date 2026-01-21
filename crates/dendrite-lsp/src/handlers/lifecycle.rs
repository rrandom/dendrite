use crate::state::GlobalState;
use dendrite_core::{DendriteEngine, DendronModel, Workspace};
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
            // 0. Update initial LSP settings if provided
            if let Some(options) = params.initialization_options {
                if let Some(dendrite_opts) = options.get("dendrite") {
                    if let Ok(new_settings) =
                        serde_json::from_value::<crate::config::LspSettings>(dendrite_opts.clone())
                    {
                        let mut config_lock = state.config.write().await;
                        *config_lock = new_settings;
                    }
                }
            }

            client
                .log_message(
                    MessageType::INFO,
                    format!("Initializing workspace at: {:?}", root_path),
                )
                .await;

            let root_path_clone = root_path.clone();
            let fs = state.fs.clone();
            let (engine, _files, stats, cache_loaded_msg) =
                tokio::task::spawn_blocking(move || {
                    // 1. Find and load config
                    let dendrite_yaml = root_path_clone.join("dendrite.yaml");
                    let config = if dendrite_yaml.exists() {
                        let content = std::fs::read_to_string(dendrite_yaml).unwrap_or_default();
                        dendrite_core::DendriteConfig::from_yaml(&content).unwrap_or_default()
                    } else {
                        let mut c = dendrite_core::DendriteConfig::default();
                        if let Some(main) = c.workspace.vaults.iter_mut().find(|v| v.name == "main") {
                            main.path = root_path_clone.clone();
                        }
                        c
                    };

                    let workspace = Workspace::new(
                        config,
                        Box::new(DendronModel::new(root_path_clone.clone())),
                    );
                    let mut v = DendriteEngine::new(workspace, fs);

                    // Try to load cache first
                    let cache_path = root_path_clone.join(".dendrite").join("cache.bin");
                    let cache_msg = match v.load_cache(&cache_path) {
                        Ok(_) => "💾 Persistent cache loaded successfully".to_string(),
                        Err(e) => format!("🆕 Starting fresh scan (cache: {})", e),
                    };

                    let (files, stats) = v.initialize(root_path_clone.clone());

                    // Save cache immediately to warm it up
                    let _ = v.save_cache(&cache_path);

                    (v, files, stats, cache_msg)
                })
                .await
                .map_err(|e| tower_lsp::jsonrpc::Error {
                    code: tower_lsp::jsonrpc::ErrorCode::InternalError,
                    message: format!("Failed to initialize workspace: {}", e).into(),
                    data: None,
                })?;

            client
                .log_message(MessageType::INFO, cache_loaded_msg)
                .await;

            client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "✅ Workspace initialized: {} files found, {} hits (T1: {}, T2: {}), {} parsed",
                        stats.total_files,
                        stats.tier1_hits + stats.tier2_hits,
                        stats.tier1_hits,
                        stats.tier2_hits,
                        stats.full_parses
                    ),
                )
                .await;

            let notes_count = engine.workspace.all_notes().len();
            client
                .log_message(
                    MessageType::INFO,
                    format!("✅ Parsed {} notes from workspace", notes_count),
                )
                .await;

            let mut engine_lock = state.engine.write().await;
            *engine_lock = Some(engine);
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
