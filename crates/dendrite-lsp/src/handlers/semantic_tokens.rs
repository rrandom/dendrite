use crate::state::GlobalState;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::KEYWORD,
];

pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[];

pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

pub async fn handle_semantic_tokens_full(
    _client: &tower_lsp::Client,
    state: &GlobalState,
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let uri = &params.text_document.uri;
    let Ok(path) = uri.to_file_path() else {
        return Ok(None);
    };

    let state_lock = state.vault.read().await;
    let Some(vault) = &*state_lock else {
        return Ok(None);
    };

    let Some(note) = vault.workspace.note_by_path(&path) else {
        return Ok(None);
    };

    let mut tokens = Vec::new();
    let mut last_line = 0;
    let mut last_start = 0;

    // Collect all links and sort them by position
    let mut links = note.links.clone();
    links.sort_by(|a, b| {
        a.range
            .start
            .line
            .cmp(&b.range.start.line)
            .then(a.range.start.col.cmp(&b.range.start.col))
    });

    for link in links {
        let line = link.range.start.line;
        let start = link.range.start.col;

        // Calculate length in characters (this is rough for UTF-8 vs UTF-16,
        // but for wikilinks without complex unicode it should be okay)
        // Ideally we need the byte length to character length conversion.
        let length = if link.range.end.line == link.range.start.line {
            (link.range.end.col - link.range.start.col) as u32
        } else {
            // Multiline links are rare in wikilinks but possible.
            // For now, we only highlight the first line to be safe.
            // A better solution would handle multiline tokens.
            10 // Placeholder or truncate
        };

        let delta_line = line - last_line;
        let delta_start = if delta_line == 0 {
            start - last_start
        } else {
            start
        };

        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: 0, // PARAMETER
            token_modifiers_bitset: 0,
        });

        last_line = line;
        last_start = start;
    }

    if tokens.is_empty() {
        return Ok(None);
    }

    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: tokens,
    })))
}
