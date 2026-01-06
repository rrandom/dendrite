#[cfg(test)]
mod tests {
    use crate::handlers;
    use crate::state::GlobalState;
    use crate::Backend;
    use dendrite_core::workspace::vfs::PhysicalFileSystem;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower_lsp::lsp_types::*;
    use tower_lsp::LspService;

    async fn setup_test_context() -> (GlobalState, TempDir, tower_lsp::Client) {
        let fs = Arc::new(PhysicalFileSystem);
        let (service, _) = LspService::new(|client| Backend::new(client, fs.clone()));
        let client = service.inner().client.clone();
        let state = service.inner().state.clone();
        let temp_dir = TempDir::new().unwrap();

        (state, temp_dir, client)
    }

    #[allow(deprecated)]
    fn create_initialize_params(root_uri: Url) -> InitializeParams {
        InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: Some(root_uri),
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: None,
            workspace_folders: None,
            client_info: None,
            locale: None,
        }
    }

    #[tokio::test]
    async fn test_lsp_initialize() {
        let (state, temp_dir, client) = setup_test_context().await;

        // Create some test notes
        let note_path = temp_dir.path().join("root.md");
        fs::write(&note_path, "# Root Note\n\n[[child]]").unwrap();

        let child_path = temp_dir.path().join("child.md");
        fs::write(&child_path, "# Child Note").unwrap();

        let params = create_initialize_params(Url::from_file_path(temp_dir.path()).unwrap());

        // Call initialize handler
        let result = handlers::handle_initialize(&client, &state, params)
            .await
            .unwrap();

        assert!(result.capabilities.completion_provider.is_some());

        // Check if vault was initialized in state
        let vault_lock = state.vault.read().await;
        assert!(vault_lock.is_some());
        let vault = vault_lock.as_ref().unwrap();
        let ws = &vault.workspace;

        assert!(ws.all_notes().len() >= 2);
    }

    #[tokio::test]
    async fn test_lsp_completion() {
        let (state, temp_dir, client) = setup_test_context().await;

        let params = create_initialize_params(Url::from_file_path(temp_dir.path()).unwrap());
        handlers::handle_initialize(&client, &state, params)
            .await
            .unwrap();

        let note_path = temp_dir.path().join("main.md");
        let content = "Check this: [[";
        fs::write(&note_path, content).unwrap();

        let uri = Url::from_file_path(&note_path).unwrap();

        handlers::handle_did_open(
            &state,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 0,
                    text: content.to_string(),
                },
            },
        )
        .await;

        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 14,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let response = handlers::handle_completion(&client, &state, completion_params)
            .await
            .unwrap();

        if let Some(CompletionResponse::Array(items)) = response {
            assert!(!items.is_empty());
            assert!(items.iter().any(|i| i.label == "main"));
        } else {
            panic!("Expected completion array");
        }
    }

    #[tokio::test]
    async fn test_lsp_goto_definition() {
        let (state, temp_dir, client) = setup_test_context().await;

        let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
        let params = create_initialize_params(root_uri);
        handlers::handle_initialize(&client, &state, params)
            .await
            .unwrap();

        // 1. Create target note
        let target_path = temp_dir.path().join("target.md");
        let target_content = "# Target Note";
        fs::write(&target_path, target_content).unwrap();
        let target_uri = Url::from_file_path(&target_path).unwrap();

        // 2. Create source note with link
        let source_path = temp_dir.path().join("source.md");
        let source_content = "Go to [[target]]";
        fs::write(&source_path, source_content).unwrap();
        let source_uri = Url::from_file_path(&source_path).unwrap();

        // Populate workspace using standard LSP notifications
        handlers::handle_did_open(
            &state,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: target_uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 0,
                    text: target_content.to_string(),
                },
            },
        )
        .await;

        handlers::handle_did_open(
            &state,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: source_uri.clone(),
                    language_id: "markdown".to_string(),
                    version: 0,
                    text: source_content.to_string(),
                },
            },
        )
        .await;

        // 3. Request definition at [[target]]
        let definition_params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: source_uri.clone(),
                },
                position: Position {
                    line: 0,
                    character: 10, // "Go to [[t" -> 6 chars for "Go to ", 2 for "[[", 1 for "t"
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response = handlers::handle_goto_definition(&client, &state, definition_params)
            .await
            .unwrap();

        match response {
            Some(GotoDefinitionResponse::Scalar(location)) => {
                assert_eq!(location.uri, target_uri);
            }
            _ => panic!("Expected scalar location response to {:?}", target_uri),
        }
    }
}
