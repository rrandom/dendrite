use crate::conversion::edit_plan_to_workspace_edit;
use crate::state::GlobalState;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

pub async fn handle_rename(
    _client: &tower_lsp::Client,
    state: &GlobalState,
    params: RenameParams,
) -> Result<Option<WorkspaceEdit>> {
    let uri = params.text_document_position.text_document.uri;
    let new_name = params.new_name;

    // Resolve workspace
    let vault = state.vault.read().await;
    let workspace = match &*vault {
        Some(v) => &v.workspace,
        None => return Ok(None),
    };

    // Resolve path from URI
    let path = match uri.to_file_path() {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };

    // Resolve Note Key from path
    let old_key = match workspace.resolve_note_key(&path) {
        Some(k) => k,
        None => return Ok(None),
    };

    // Calculate edits using Vault API
    // We use match instead of ? to handle Option->Result correctly
    let plan = match &*vault {
        Some(v) => v.rename_note(&old_key, &new_name),
        None => return Ok(None),
    };

    match plan {
        Some(p) => Ok(Some(edit_plan_to_workspace_edit(p))),
        None => Ok(None),
    }
}
