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
    let engine = state.engine.read().await;
    let workspace = match &*engine {
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
    let plan = match &*engine {
        Some(v) => v.rename_note(&old_key, &new_name),
        None => return Ok(None),
    };

    match plan {
        Some(p) => {
            if p.reversible {
                let limit = {
                    let config = state.config.read().await;
                    config.mutation_history_limit
                };
                let mut history = state.mutation_history.write().await;
                history.push_back(p.clone());
                // Limit history size based on config
                while history.len() > limit {
                    history.pop_front();
                }
            }
            Ok(Some(edit_plan_to_workspace_edit(p)))
        }
        None => Ok(None),
    }
}
