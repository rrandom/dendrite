use crate::state::GlobalState;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

/// Handle "textDocument/completion" request
pub async fn handle_completion(
    _client: &Client,
    state: &GlobalState,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    // 1. Get document content
    let document_text = {
        let cache = state.document_cache.read().await;
        cache.get(uri).cloned()
    };

    let Some(document_text) = document_text else {
        return Ok(None);
    };

    // 2. Extract text before cursor safely (handling UTF-16 based LSP position)
    let line_idx = position.line as usize;
    let char_idx = position.character as usize; // LSP uses UTF-16 code units

    let lines: Vec<&str> = document_text.lines().collect();
    if line_idx >= lines.len() {
        return Ok(None);
    }
    let current_line = lines[line_idx];

    // Convert UTF-16 offset to character offset
    // Note: This is an approximation. Ideally we convert UTF-16 offset -> Rust Char Index -> Byte Index
    // For now, we assume 1 LSP char = 1 Rust char (which holds for BMP).
    // Surrogates require proper handling, but let's just use chars().take() safely.
    let text_before: String = current_line.chars().take(char_idx).collect();

    // 3. Check for [[ context by searching backwards
    let Some(last_open_idx) = text_before.rfind("[[") else {
        return Ok(None);
    };

    // Verify no closing ]] strictly *after* the [[ and *before* cursor
    if text_before[last_open_idx..].contains("]]") {
        return Ok(None);
    }

    // Input: anything after [[
    let input_raw = &text_before[last_open_idx + 2..];

    // 4. Initialize Vault
    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        return Ok(None);
    };
    let ws = &vault.workspace;

    // 5. Determine completion mode (Note vs Anchor)
    let items = if let Some((note_part, _anchor_part)) = input_raw.split_once('#') {
        // --- ANCHOR COMPLETION ---
        let target_note = if note_part.is_empty() {
            let current_path = uri.to_file_path().ok();
            current_path.and_then(|p| ws.note_by_path(&p))
        } else {
            ws.lookup_note(note_part)
        };

        if let Some(note) = target_note {
            let mut items = Vec::new();
            for heading in &note.headings {
                let label = heading.text.clone();
                let insert_text = dendrite_core::slugify_heading(&label);
                items.push(CompletionItem {
                    label,
                    kind: Some(CompletionItemKind::CLASS),
                    insert_text: Some(insert_text),
                    filter_text: Some(format!("#{}", heading.text)),
                    detail: Some(format!("Heading H{}", heading.level)),
                    ..Default::default()
                });
            }
            for block in &note.blocks {
                items.push(CompletionItem {
                    label: format!("^{}", block.id),
                    kind: Some(CompletionItemKind::FIELD),
                    insert_text: Some(format!("^{}", block.id)),
                    detail: Some("Block Anchor".to_string()),
                    ..Default::default()
                });
            }
            items
        } else {
            Vec::new()
        }
    } else {
        // --- NOTE COMPLETION ---
        ws.all_note_keys()
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
            .collect()
    };

    Ok(Some(CompletionResponse::Array(items)))
}
