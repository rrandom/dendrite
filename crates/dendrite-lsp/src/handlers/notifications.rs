use crate::state::GlobalState;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

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
            let _ = state.dirty_signal.send(());
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
                let _ = state.dirty_signal.send(());
            }
        }
    }
}

/// Handle "workspace/didChangeWatchedFiles" notification
pub async fn handle_did_change_watched_files(
    client: &Client,
    state: &GlobalState,
    params: DidChangeWatchedFilesParams,
) {
    let mut vault_lock = state.vault.write().await;
    let mut changed = false;

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
                            changed = true;
                        }
                    }
                    FileChangeType::DELETED => {
                        // Remove from cache
                        {
                            let mut cache = state.document_cache.write().await;
                            cache.remove(&uri);
                        }
                        v.delete_file(&path);
                        changed = true;
                    }
                    _ => {}
                }
            }
        }
    }

    if changed {
        let _ = state.dirty_signal.send(());
        client
            .send_notification::<HierarchyChangedNotification>(serde_json::Value::Null)
            .await;
    }
}

struct HierarchyChangedNotification;

impl tower_lsp::lsp_types::notification::Notification for HierarchyChangedNotification {
    type Params = serde_json::Value;
    const METHOD: &'static str = "dendrite/hierarchyChanged";
}

/// Handle "workspace/didRenameFiles" notification
pub async fn handle_did_rename_files(
    client: &Client,
    state: &GlobalState,
    params: RenameFilesParams,
) {
    let mut vault_lock = state.vault.write().await;
    if let Some(v) = &mut *vault_lock {
        for file_rename in params.files {
            let old_uri = file_rename.old_uri.parse::<Url>();
            let new_uri = file_rename.new_uri.parse::<Url>();

            if let (Ok(old_url), Ok(new_url)) = (old_uri, new_uri) {
                if let (Ok(old_path), Ok(new_path)) =
                    (old_url.to_file_path(), new_url.to_file_path())
                {
                    // Read content of the new file
                    if let Ok(content) = state.fs.read_to_string(&new_path) {
                        // Update cache for the new URI
                        {
                            let mut cache = state.document_cache.write().await;
                            cache.insert(new_url.clone(), content.clone());
                            cache.remove(&old_url);
                        }

                        // 1. Update internal index
                        v.rename_file(old_path.clone(), new_path.clone(), &content);

                        // 2. Generate mutation plan for the move
                        if let Some(mut plan) = v.move_note(&old_path, new_path) {
                            // 3. Filter out the RenameFile operation (it's already done by the user in the IDE)
                            plan.edits.retain(|group| {
                                group.changes.iter().all(|change| {
                                    !matches!(
                                        change,
                                        dendrite_core::mutation::model::Change::ResourceOp(_)
                                    )
                                })
                            });

                            if !plan.edits.is_empty() {
                                // 4. Convert and apply edits
                                let workspace_edit =
                                    crate::conversion::edit_plan_to_workspace_edit(plan.clone());
                                let _ = client.apply_edit(workspace_edit).await;

                                // Store in history for undo
                                if plan.reversible {
                                    let mut history = state.mutation_history.write().await;
                                    history.push_back(plan);
                                    if history.len() > 5 {
                                        history.pop_front();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = state.dirty_signal.send(());
    }
}
