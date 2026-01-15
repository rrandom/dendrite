use crate::state::GlobalState;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::{ExecuteCommandParams, MessageType, Url};
use tower_lsp::Client;

use crate::protocol::{GetHierarchyParams, GetHierarchyResult};

/// Handle "dendrite/getHierarchy" request
/// Returns the complete hierarchy tree structure including Ghost Nodes
pub async fn handle_get_hierarchy(
    client: &Client,
    state: &GlobalState,
    _params: GetHierarchyParams,
) -> Result<GetHierarchyResult> {
    client
        .log_message(
            MessageType::INFO,
            "🌳 GetHierarchy request received".to_string(),
        )
        .await;

    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        client
            .log_message(
                MessageType::WARNING,
                "⚠️ Vault not initialized for getHierarchy".to_string(),
            )
            .await;
        return Err(Error {
            code: ErrorCode::InternalError,
            message: "Vault not initialized".into(),
            data: None,
        });
    };
    let ws = &vault.workspace;

    // Get tree view from workspace
    let tree_view = ws.get_tree_view();

    client
        .log_message(
            MessageType::INFO,
            format!("🌳 Returning {} root nodes", tree_view.len()),
        )
        .await;

    Ok(GetHierarchyResult { roots: tree_view })
}

/// Handle "dendrite/reorganizeHierarchy" command
/// Arguments: [old_key, new_key]
pub async fn handle_reorganize_hierarchy_command(
    client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    let (old_key, new_key) = parse_hierarchy_args(&params)?;

    let vault_guard = state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    let plan = vault.rename_hierarchy(&old_key, &new_key);

    if let Some(plan) = plan {
        crate::handlers::refactor::apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::Value::Bool(true)))
    } else {
        Ok(Some(serde_json::Value::Bool(false)))
    }
}

/// Handle "dendrite/resolveHierarchyEdits" command
/// Arguments: [old_key, new_key]
/// Returns: [[OldKey, NewKey], ...]
pub async fn handle_resolve_hierarchy_edits(
    _client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    let (old_key, new_key) = parse_hierarchy_args(&params)?;

    let vault_guard = state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    // Dry Run
    let plan = vault.rename_hierarchy(&old_key, &new_key);
    let mut moves = Vec::new();

    if let Some(plan) = plan {
        for group in plan.edits {
            for change in group.changes {
                if let dendrite_core::refactor::model::Change::ResourceOp(
                    dendrite_core::refactor::model::ResourceOperation::RenameFile { new_uri, .. },
                ) = change
                {
                    let old_uri = &group.uri;
                    
                    // Helper to resolve URI or Path to PathBuf
                    let to_path = |s: &str| -> Option<std::path::PathBuf> {
                        if let Ok(u) = Url::parse(s) {
                            if u.scheme() == "file" {
                                return u.to_file_path().ok();
                            }
                        }
                        // Try as raw path
                        let p = std::path::PathBuf::from(s);
                        if p.is_absolute() {
                            Some(p)
                        } else {
                            Some(p)
                        }
                    };

                    if let (Some(op), Some(np)) = (to_path(old_uri), to_path(&new_uri)) {
                         let k1 = vault.workspace.resolve_note_key(&op).unwrap_or_default();
                         let k2 = vault.workspace.resolve_note_key(&np).unwrap_or_default();
                         moves.push((k1, k2));
                    }
                }
            }
        }
    }

    Ok(Some(serde_json::to_value(moves).unwrap()))
}

fn parse_hierarchy_args(params: &ExecuteCommandParams) -> Result<(String, String)> {
    if params.arguments.len() < 2 {
        return Err(Error::invalid_params(
            "Missing arguments: [old_key, new_key]",
        ));
    }

    let old_key: String = serde_json::from_value(params.arguments[0].clone())
        .map_err(|_| Error::invalid_params("Invalid old_key"))?;
    let new_key: String = serde_json::from_value(params.arguments[1].clone())
        .map_err(|_| Error::invalid_params("Invalid new_key"))?;

    Ok((old_key, new_key))
}
