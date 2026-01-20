use crate::protocol::{GetBacklinksResult, NoteSummary};
use crate::state::GlobalState;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;

pub async fn handle_get_backlinks_command(
    _state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    let params: crate::protocol::GetBacklinksParams =
        if let Some(first_arg) = params.arguments.first() {
            serde_json::from_value(first_arg.clone())
                .map_err(|_| Error::invalid_params("Invalid params"))?
        } else {
            return Err(Error::invalid_params("Missing params"));
        };
    let note_key = params.note_key;

    let vault_guard = _state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    let backlinks = vault
        .workspace
        .backlinks_by_key(&note_key)
        .iter()
        .filter_map(|note| {
            let key = vault.workspace.key_of_note(note)?;
            let title = Some(vault.workspace.display_name(note));
            let uri = note.path.as_ref().map(|p| p.to_string_lossy().to_string());

            Some(NoteSummary { key, uri, title })
        })
        .collect::<Vec<_>>();

    let result = GetBacklinksResult { backlinks };
    serde_json::to_value(result).map(Some).map_err(|e| Error {
        code: tower_lsp::jsonrpc::ErrorCode::InternalError,
        message: format!("Failed to serialize results: {}", e).into(),
        data: None,
    })
}
