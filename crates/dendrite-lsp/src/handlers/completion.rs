use crate::state::GlobalState;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

/// Handle "textDocument/completion" request
pub async fn handle_completion(
    client: &Client,
    state: &GlobalState,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    client
        .log_message(
            MessageType::INFO,
            format!(
                "🔍 Completion requested at {:?}",
                params.text_document_position.position
            ),
        )
        .await;

    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        client
            .log_message(MessageType::WARNING, "⚠️ Vault not initialized")
            .await;
        return Ok(None);
    };
    let ws = &vault.workspace;

    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    client
        .log_message(
            MessageType::INFO,
            format!(
                "📄 URI: {:?}, Position: line {}, char {}",
                uri, position.line, position.character
            ),
        )
        .await;

    // Get document content from cache
    let document_text = {
        let cache = state.document_cache.read().await;
        cache.get(uri).cloned()
    };

    let Some(document_text) = document_text else {
        client
            .log_message(
                MessageType::WARNING,
                format!("❌ Document not found in cache: {:?}", uri),
            )
            .await;
        return Ok(None);
    };

    // Check if we're in a [[ context
    let line_idx = position.line as usize;
    let char_idx = position.character as usize;

    let lines: Vec<&str> = document_text.lines().collect();
    if line_idx >= lines.len() {
        return Ok(None);
    }

    let current_line = lines[line_idx];

    if char_idx < 2 {
        return Ok(None);
    }

    let chars_before: Vec<char> = current_line
        .chars()
        .take(char_idx)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .take(3)
        .collect();

    if chars_before.len() < 2 {
        return Ok(None);
    }

    let is_double_bracket = chars_before[0] == '[' && chars_before[1] == '[';

    if !is_double_bracket {
        return Ok(None);
    }

    if chars_before.len() >= 3 && chars_before[2] == '[' {
        return Ok(None);
    }

    // Get all note keys for completion
    let note_keys = ws.all_note_keys();

    // Create completion items
    let items: Vec<CompletionItem> = note_keys
        .into_iter()
        .map(|(key, display_name)| {
            let detail = if display_name.is_empty() {
                None
            } else {
                Some(display_name)
            };

            CompletionItem {
                label: key.clone(),
                kind: Some(CompletionItemKind::FILE),
                detail,
                insert_text: Some(key),
                ..Default::default()
            }
        })
        .collect();

    Ok(Some(CompletionResponse::Array(items)))
}
