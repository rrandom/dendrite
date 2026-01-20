use crate::state::GlobalState;
use dendrite_core::model::TextRange;
use dendrite_core::mutation::model::EditPlan;

use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

/// Helper to apply EditPlan via WorkspaceEdit
pub(crate) async fn apply_edit_plan(client: &Client, plan: EditPlan) -> Result<()> {
    let workspace_edit = crate::conversion::edit_plan_to_workspace_edit(plan);

    client
        .apply_edit(workspace_edit)
        .await?
        .applied
        .then_some(())
        .ok_or_else(|| Error {
            code: ErrorCode::InternalError,
            message: "Client failed to apply workspace edit".into(),
            data: None,
        })
}

pub async fn handle_code_action(
    _client: &Client,
    _state: &GlobalState,
    params: CodeActionParams,
) -> Result<Option<Vec<CodeActionOrCommand>>> {
    // Only support RefactorExtract for now
    let mut actions = Vec::new();

    // Check if there is a selection (non-empty range)
    let range = params.range;
    if range.start != range.end {
        // Offer "Extract to New Note"
        let split_action = CodeAction {
            title: "Refactor: Extract to New Note".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            command: Some(Command {
                title: "Extract to New Note".to_string(),
                command: "dendrite.splitNote".to_string(), // Client -> Server command
                arguments: Some(vec![
                    serde_json::to_value(params.text_document.uri).unwrap(),
                    serde_json::to_value(range).unwrap(),
                ]),
            }),
            ..Default::default()
        };
        actions.push(CodeActionOrCommand::CodeAction(split_action));
    }

    Ok(Some(actions))
}

pub async fn handle_create_note(
    client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    // 1. Parse Arguments
    if params.arguments.is_empty() {
        return Err(Error {
            code: ErrorCode::InvalidParams,
            message: "Missing argument: note_key".into(),
            data: None,
        });
    }

    let note_key: String = serde_json::from_value(params.arguments[0].clone())
        .map_err(|_| Error::invalid_params("Invalid note_key argument"))?;

    // 2. Access Vault
    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        return Err(Error {
            code: ErrorCode::InternalError,
            message: "Vault not initialized".into(),
            data: None,
        });
    };

    let plan = vault.create_note(&note_key);

    // 4. Apply changes (if any)
    if let Some(plan) = plan {
        // Extract the target URI and ensure it's a properly formatted URI string
        let target_uri = plan.edits.first().and_then(|e| {
            // Try as URL first, fallback to path-to-uri
            tower_lsp::lsp_types::Url::parse(&e.uri)
                .ok()
                .filter(|u| u.scheme() == "file")
                .or_else(|| {
                    tower_lsp::lsp_types::Url::from_file_path(std::path::PathBuf::from(&e.uri)).ok()
                })
                .map(|u| u.to_string())
        });

        // Reuse existing mutation handler to apply EditPlan
        apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::to_value(target_uri).unwrap()))
    } else {
        Ok(None)
    }
}

pub async fn handle_split_note_command(
    client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    // Arguments: [source_uri, range, new_note_name]
    if params.arguments.len() < 3 {
        return Err(Error {
            code: ErrorCode::InvalidParams,
            message: "Missing arguments: [source_uri, range, new_note_name]".into(),
            data: None,
        });
    }

    let source_uri: Url = serde_json::from_value(params.arguments[0].clone())
        .map_err(|_| Error::invalid_params("Invalid source_uri"))?;
    let range: Range = serde_json::from_value(params.arguments[1].clone())
        .map_err(|_| Error::invalid_params("Invalid range"))?;
    let new_note_name: String = serde_json::from_value(params.arguments[2].clone())
        .map_err(|_| Error::invalid_params("Invalid new_note_name"))?;

    let vault_guard = state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    let source_path = source_uri
        .to_file_path()
        .map_err(|_| Error::internal_error())?;

    // Verify key exists
    let _old_key = vault
        .workspace
        .resolve_note_key(&source_path)
        .ok_or_else(|| Error {
            code: ErrorCode::InvalidParams,
            message: format!("Could not find note key for path: {:?}", source_path).into(),
            data: None,
        })?;

    // Convert Range to TextRange
    let text_range = TextRange {
        start: dendrite_core::model::Point {
            line: range.start.line,
            col: range.start.character,
        },
        end: dendrite_core::model::Point {
            line: range.end.line,
            col: range.end.character,
        },
    };

    let plan = vault.split_note(&source_path, text_range, &new_note_name);

    if let Some(plan) = plan {
        apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::Value::Bool(true)))
    } else {
        Ok(Some(serde_json::Value::Bool(false)))
    }
}

pub async fn handle_delete_note_command(
    client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    let params: crate::protocol::DeleteNoteParams =
        if let Some(first_arg) = params.arguments.first() {
            serde_json::from_value(first_arg.clone())
                .map_err(|_| Error::invalid_params("Invalid params"))?
        } else {
            return Err(Error::invalid_params("Missing params"));
        };
    let note_key = params.note_key;

    let vault_guard = state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    let plan = vault.delete_note(&note_key);

    if let Some(plan) = plan {
        apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::Value::Bool(true)))
    } else {
        Ok(Some(serde_json::Value::Bool(false)))
    }
}

pub async fn handle_undo_mutation(client: &tower_lsp::Client, state: &GlobalState) -> Result<()> {
    let mut history = state.mutation_history.write().await;

    if let Some(plan) = history.pop_back() {
        // 1. Invert the plan
        let vault_guard = state.vault.read().await;
        // Map Vault (if present) to dyn ContentProvider
        let cp = vault_guard
            .as_ref()
            .map(|v| v as &dyn dendrite_core::mutation::model::ContentProvider);
        let inverted_plan = plan.invert(cp);

        // 2. Convert to WorkspaceEdit
        let edit = crate::conversion::edit_plan_to_workspace_edit(inverted_plan);

        // 3. Ask client to apply the edit
        match client.apply_edit(edit).await {
            Ok(response) if response.applied => {
                client
                    .show_message(MessageType::INFO, "Mutation undone successfully.")
                    .await;
                Ok(())
            }
            Ok(_) => Err(Error {
                code: ErrorCode::InternalError,
                message: "Undo edit was rejected by client or failed to apply.".into(),
                data: None,
            }),
            Err(e) => Err(Error {
                code: ErrorCode::InternalError,
                message: format!("Failed to apply undo edit: {}", e).into(),
                data: None,
            }),
        }
    } else {
        // No history to undo
        client
            .show_message(MessageType::INFO, "No mutation history to undo.")
            .await;
        Ok(())
    }
}
