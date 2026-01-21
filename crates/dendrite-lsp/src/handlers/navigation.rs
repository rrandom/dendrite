use crate::conversion::{lsp_position_to_point, path_to_uri, text_range_to_lsp_range};
use crate::state::GlobalState;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

/// Handle "textDocument/definition" request
pub async fn handle_goto_definition(
    client: &Client,
    state: &GlobalState,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let state_lock = state.engine.read().await;
    let Some(engine) = &*state_lock else {
        return Ok(None);
    };
    let ws = &engine.workspace;

    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let Ok(path) = uri.to_file_path() else {
        return Ok(None);
    };

    // Convert LSP Position to Core Point
    let point = lsp_position_to_point(position);

    client
        .log_message(
            MessageType::INFO,
            format!("🔍 Seeking link at path={:?}, point={:?}", path, point),
        )
        .await;

    // Find the link at the given position
    let Some(link) = ws.find_link_at_position(&path, point) else {
        client
            .log_message(MessageType::INFO, "❌ No link found at position")
            .await;
        return Ok(None);
    };

    client
        .log_message(
            MessageType::INFO,
            format!("🔗 Found link at line {}", link.range.start.line),
        )
        .await;

    // Get the target note's path
    let Some(target_path) = ws.get_link_target_path(link) else {
        client
            .log_message(MessageType::WARNING, "⚠️ Target path not found")
            .await;
        return Ok(None);
    };

    // Convert path to URI
    let Some(target_uri) = path_to_uri(&target_path) else {
        return Ok(None);
    };

    // Return the definition location
    let target_range = ws
        .resolve_link_anchor(link)
        .map(text_range_to_lsp_range)
        .unwrap_or(Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        });

    Ok(Some(GotoDefinitionResponse::Scalar(Location {
        uri: target_uri,
        range: target_range,
    })))
}

/// Handle "textDocument/hover" request
pub async fn handle_hover(
    _client: &Client,
    state: &GlobalState,
    params: HoverParams,
) -> Result<Option<Hover>> {
    let state_lock = state.engine.read().await;
    let Some(engine) = &*state_lock else {
        return Ok(None);
    };
    let ws = &engine.workspace;

    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let Ok(path) = uri.to_file_path() else {
        return Ok(None);
    };

    // Convert LSP Position to Core Point
    let point = lsp_position_to_point(position);

    // Find the link at the given position
    let Some(link) = ws.find_link_at_position(&path, point) else {
        _client
            .log_message(
                MessageType::INFO,
                format!("❌ No link found for hover at point={:?}", point),
            )
            .await;
        return Ok(None);
    };

    // Get the target note for hover information
    let target_path = ws.get_link_target_path(link);
    let target_node = target_path.as_ref().and_then(|path| ws.note_by_path(path));

    let target_info = if let (Some(path), Some(note)) = (&target_path, target_node) {
        // Read the file accurately
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Determine the starting point: Anchor/Block range or Content Offset
                let lines: Vec<&str> = content.lines().collect();
                let start_line = if let Some(range) = ws.resolve_link_anchor(link) {
                    range.start.line
                } else {
                    // Find the line where note.content_offset falls
                    let mut current_offset = 0u32;
                    let mut found_line = 0u32;
                    for (i, line) in lines.iter().enumerate() {
                        if current_offset >= note.content_offset {
                            found_line = i as u32;
                            break;
                        }
                        current_offset += (line.len() + 1) as u32; // +1 for newline
                    }
                    found_line
                };

                let preview: String = lines
                    .iter()
                    .skip(start_line as usize)
                    .take(10)
                    .cloned()
                    .collect::<Vec<&str>>()
                    .join("\n");

                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
                format!("```{}\n{}\n```", extension, preview.trim())
            }
            Err(_) => format!("Target: {:?}", path),
        }
    } else if let Some(path) = &target_path {
        format!("Target: {:?}", path)
    } else {
        "Target: (not found)".to_string()
    };

    // Convert link range to LSP range for hover highlighting
    let link_range = text_range_to_lsp_range(link.range);

    // Return hover with the link range for proper highlighting
    Ok(Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: target_info,
        }),
        range: Some(link_range),
    }))
}

/// Handle "textDocument/documentHighlight" request
pub async fn handle_document_highlight(
    client: &Client,
    state: &GlobalState,
    params: DocumentHighlightParams,
) -> Result<Option<Vec<DocumentHighlight>>> {
    client
        .log_message(
            MessageType::INFO,
            format!(
                "✨ Document highlight requested at {:?}",
                params.text_document_position_params.position
            ),
        )
        .await;

    let state_lock = state.engine.read().await;
    let Some(engine) = &*state_lock else {
        return Ok(None);
    };
    let ws = &engine.workspace;

    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let Ok(path) = uri.to_file_path() else {
        return Ok(None);
    };

    // Convert LSP Position to Core Point
    let point = lsp_position_to_point(position);

    // Find the link at the given position
    let Some(link) = ws.find_link_at_position(&path, point) else {
        client
            .log_message(
                MessageType::INFO,
                format!("❌ No link found for highlight at point={:?}", point),
            )
            .await;
        return Ok(None);
    };

    // Convert link range to LSP range for highlighting
    let link_range = text_range_to_lsp_range(link.range);

    client
        .log_message(
            MessageType::INFO,
            format!(
                "✨ Highlighting link at range {:?}-{:?}",
                link_range.start, link_range.end
            ),
        )
        .await;

    // Return the highlight
    Ok(Some(vec![DocumentHighlight {
        range: link_range,
        kind: Some(DocumentHighlightKind::TEXT),
    }]))
}
