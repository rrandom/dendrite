use crate::conversion::edit_plan_to_workspace_edit;
use crate::state::GlobalState;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::MessageType;

pub async fn handle_undo_mutation(client: &tower_lsp::Client, state: &GlobalState) -> Result<()> {
    let mut history = state.mutation_history.write().await;

    if let Some(plan) = history.pop_back() {
        // 1. Invert the plan
        let inverted_plan = plan.invert();

        // 2. Convert to WorkspaceEdit
        let edit = edit_plan_to_workspace_edit(inverted_plan);

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
