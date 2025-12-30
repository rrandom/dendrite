use crate::protocol::{ListNotesParams, ListNotesResult};
use tower_lsp::jsonrpc::Result;

/// Handle "dendrite/listNotes" request
/// TODO: Implement after Week 4 when Store has search functionality
#[allow(dead_code)]
pub async fn handle_list_notes(_params: ListNotesParams) -> Result<ListNotesResult> {
    Ok(ListNotesResult { notes: vec![] })
}
