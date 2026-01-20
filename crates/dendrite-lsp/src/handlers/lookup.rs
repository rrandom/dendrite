use crate::conversion::path_to_uri;
use crate::protocol::{GetBacklinksResult, ListNotesParams, ListNotesResult, NoteSummary};
use crate::state::GlobalState;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::{ExecuteCommandParams, MessageType};
use tower_lsp::Client;

/// Handle "dendrite/listNotes" request
/// Returns a list of all notes, optionally filtered by query
pub async fn handle_list_notes(
    client: &Client,
    state: &GlobalState,
    params: ListNotesParams,
) -> Result<ListNotesResult> {
    client
        .log_message(
            MessageType::INFO,
            format!("📋 ListNotes request received (query: {:?})", params.query),
        )
        .await;

    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        client
            .log_message(
                MessageType::WARNING,
                "⚠️ Vault not initialized for listNotes".to_string(),
            )
            .await;
        return Err(Error {
            code: ErrorCode::InternalError,
            message: "Vault not initialized".into(),
            data: None,
        });
    };
    let ws = &vault.workspace;

    // Get all notes
    let all_notes = ws.all_notes();

    // Get all note keys and display names, create a map from NoteId to (key, display_name)
    // We need to match notes by NoteId, so we'll create a temporary mapping
    let all_note_keys = ws.all_note_keys();

    // Create a map from key to display_name for quick lookup
    let key_to_display_name: std::collections::HashMap<String, String> = all_note_keys
        .iter()
        .map(|(key, display_name)| (key.clone(), display_name.clone()))
        .collect();

    // Convert to NoteSummary
    let mut note_summaries: Vec<NoteSummary> = all_notes
        .iter()
        .filter_map(|note| {
            // Try to get note key from path (filename without extension)
            // This matches the DendronStrategy::note_key_from_path implementation
            let note_key = note.path.as_ref().and_then(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|s| s.to_string())
            });

            // If no path, try to find key from all_note_keys by matching NoteId
            // Since all_note_keys() iterates through all notes, we can't directly match by NoteId
            // So we'll use the path-based key for notes with paths, and skip Ghost Nodes without keys
            let note_key = note_key?;

            // Get display name from map, fallback to note.title
            let display_name = key_to_display_name
                .get(&note_key)
                .cloned()
                .unwrap_or_else(|| note.title.clone().unwrap_or_default());

            // Get URI from path
            let uri = note
                .path
                .as_ref()
                .and_then(|path| path_to_uri(path).map(|u| u.to_string()));

            Some(NoteSummary {
                key: note_key,
                uri,
                title: if display_name.is_empty() {
                    None
                } else {
                    Some(display_name)
                },
            })
        })
        .collect();

    // Apply query filter if provided
    if let Some(query) = &params.query {
        let query_lower = query.to_lowercase();
        note_summaries.retain(|summary| {
            // Filter by title or key (case-insensitive)
            summary
                .title
                .as_ref()
                .map(|t| t.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
                || summary.key.to_lowercase().contains(&query_lower)
        });
    }

    client
        .log_message(
            MessageType::INFO,
            format!("📋 Returning {} notes", note_summaries.len()),
        )
        .await;

    Ok(ListNotesResult {
        notes: note_summaries,
    })
}

/// Handle "dendrite/getNoteKey" request
/// Returns the NoteId for a given file URI
pub async fn handle_get_note_key(
    client: &Client,
    state: &GlobalState,
    params: crate::protocol::GetNoteKeyParams,
) -> Result<crate::protocol::GetNoteKeyResult> {
    client
        .log_message(
            MessageType::INFO,
            format!("🔍 GetNoteKey request received for: {}", params.uri),
        )
        .await;

    let uri = tower_lsp::lsp_types::Url::parse(&params.uri).map_err(|e| Error {
        code: ErrorCode::InvalidParams,
        message: format!("Invalid URI: {}", e).into(),
        data: None,
    })?;

    let path = uri.to_file_path().map_err(|_| Error {
        code: ErrorCode::InvalidParams,
        message: "URI is not a file path".into(),
        data: None,
    })?;

    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        return Err(Error {
            code: ErrorCode::InternalError,
            message: "Vault not initialized".into(),
            data: None,
        });
    };

    let key = vault
        .workspace
        .resolve_note_key(&path)
        .ok_or_else(|| Error {
            code: ErrorCode::InvalidParams,
            message: "Note not found".into(),
            data: None,
        })?;

    Ok(crate::protocol::GetNoteKeyResult { key })
}

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
