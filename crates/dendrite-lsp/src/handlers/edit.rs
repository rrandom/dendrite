use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::ExecuteCommandParams;
use tower_lsp::Client;

use crate::state::GlobalState;
// use crate::protocol::CreateNoteParams;

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
        // Reuse existing refactor handler to apply EditPlan
        crate::handlers::refactor::apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::Value::Bool(true)))
    } else {
        Ok(Some(serde_json::Value::Bool(false)))
    }
}
