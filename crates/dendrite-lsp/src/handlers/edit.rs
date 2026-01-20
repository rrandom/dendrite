use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::ExecuteCommandParams;
use dendrite_core::refactor::model::EditPlan;
use tower_lsp::Client;

use crate::state::GlobalState;

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


pub async fn handle_create_note(
    client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    // 1. Parse Arguments
    if params.arguments.len() < 1 {
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
        let target_uri = plan.edits.get(0).and_then(|e| {
            // Try as URL first, fallback to path-to-uri
            tower_lsp::lsp_types::Url::parse(&e.uri)
                .ok()
                .filter(|u| u.scheme() == "file")
                .or_else(|| {
                    tower_lsp::lsp_types::Url::from_file_path(std::path::PathBuf::from(&e.uri)).ok()
                })
                .map(|u| u.to_string())
        });

        // Reuse existing refactor handler to apply EditPlan
        crate::handlers::edit::apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::to_value(target_uri).unwrap()))
    } else {
        Ok(None)
    }
}
