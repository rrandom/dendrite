use crate::protocol::{ListNotesParams, ListNotesResult, NoteSummary};
use crate::state::GlobalState;
use crate::conversion::path_to_uri;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::MessageType;
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

    let workspace = state.workspace.read().await;
    let Some(ws) = &*workspace else {
        client
            .log_message(
                MessageType::WARNING,
                "⚠️ Workspace not initialized for listNotes".to_string(),
            )
            .await;
        return Err(Error {
            code: ErrorCode::InternalError,
            message: "Workspace not initialized".into(),
            data: None,
        });
    };

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
                .unwrap_or_else(|| {
                    note.title.clone().unwrap_or_default()
                });
            
            // Get URI from path
            let uri = note.path.as_ref().and_then(|path| {
                path_to_uri(path).map(|u| u.to_string())
            });
            
            Some(NoteSummary {
                key: note_key,
                uri,
                title: if display_name.is_empty() { None } else { Some(display_name) },
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
                || summary
                    .key
                    .to_lowercase()
                    .contains(&query_lower)
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
