use crate::state::GlobalState;
use tower_lsp::lsp_types::*;

/// Handle "textDocument/didOpen" notification
pub async fn handle_did_open(state: &GlobalState, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri.clone();
    let text = params.text_document.text.clone();

    // Update document cache
    {
        let mut cache = state.document_cache.write().await;
        cache.insert(uri.clone(), text.clone());
    }

    // Update vault
    let mut vault_lock = state.vault.write().await;
    if let Some(v) = &mut *vault_lock {
        if let Ok(path) = uri.to_file_path() {
            v.update_content(path, &text);
        }
    }
}

/// Handle "textDocument/didChange" notification
pub async fn handle_did_change(state: &GlobalState, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri.clone();

    if let Some(last_change) = params.content_changes.last() {
        let text = last_change.text.clone();

        // Update document cache
        {
            let mut cache = state.document_cache.write().await;
            cache.insert(uri.clone(), text.clone());
        }

        // Update vault
        let mut vault_lock = state.vault.write().await;
        if let Some(v) = &mut *vault_lock {
            if let Ok(path) = uri.to_file_path() {
                v.update_content(path, &text);
            }
        }
    }
}

/// Handle "workspace/didChangeWatchedFiles" notification
pub async fn handle_did_change_watched_files(
    state: &GlobalState,
    params: DidChangeWatchedFilesParams,
) {
    let mut vault_lock = state.vault.write().await;
    if let Some(v) = &mut *vault_lock {
        for change in params.changes {
            let uri = change.uri.clone();
            if let Ok(path) = uri.to_file_path() {
                match change.typ {
                    FileChangeType::CREATED | FileChangeType::CHANGED => {
                        if let Ok(content) = state.fs.read_to_string(&path) {
                            // Update cache
                            {
                                let mut cache = state.document_cache.write().await;
                                cache.insert(uri, content.clone());
                            }
                            v.update_content(path, &content);
                        }
                    }
                    FileChangeType::DELETED => {
                        // Remove from cache
                        {
                            let mut cache = state.document_cache.write().await;
                            cache.remove(&uri);
                        }
                        v.delete_file(&path);
                    }
                    _ => {}
                }
            }
        }
    }
}
