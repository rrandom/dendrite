use crate::state::GlobalState;
use dendrite_core::parser::get_updated_field_range;
use dendrite_core::utils::time;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

/// Handle "textDocument/willSaveWaitUntil" request
pub async fn handle_will_save_wait_until(
    state: &GlobalState,
    params: WillSaveTextDocumentParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = params.text_document.uri;
    let path = uri
        .to_file_path()
        .map_err(|_| tower_lsp::jsonrpc::Error::invalid_params("Invalid URI"))?;

    // 1. Get current text from cache
    let cache = state.document_cache.read().await;
    let Some(text) = cache.get(&uri) else {
        return Ok(None);
    };

    // 2. Get note metadata to find frontmatter limit
    let vault_lock = state.vault.read().await;
    let Some(vault) = &*vault_lock else {
        return Ok(None);
    };

    let Some(note) = vault.workspace.note_by_path(&path) else {
        return Ok(None);
    };

    // 3. Find updated field range
    let limit = note.content_offset as usize;
    if let Some(range) = get_updated_field_range(text, limit) {
        // 4. Generate edit
        let now = time::now();

        // We need to convert dendrite-core TextRange to LSP Range
        let lsp_range = Range {
            start: Position {
                line: range.start.line,
                character: range.start.col,
            },
            end: Position {
                line: range.end.line,
                character: range.end.col,
            },
        };

        return Ok(Some(vec![TextEdit {
            range: lsp_range,
            new_text: now.to_string(),
        }]));
    }

    Ok(None)
}
