use crate::protocol::{GetHierarchyParams, GetHierarchyResult};
use crate::state::GlobalState;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::MessageType;
use tower_lsp::Client;

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
