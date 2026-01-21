use crate::handlers;
use crate::Backend;
use dendrite_core::vfs::PhysicalFileSystem;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tower_lsp::lsp_types::*;
use tower_lsp::LanguageServer;
use tower_lsp::LspService;

async fn setup_test_context() -> (Backend, TempDir) {
    let fs = Arc::new(PhysicalFileSystem);
    let (service, _) = LspService::new(|client| Backend::new(client, fs.clone()));
    let backend = service.inner().clone();
    let temp_dir = TempDir::new().unwrap();

    (backend, temp_dir)
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
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    // Create some test notes
    let note_path = temp_dir.path().join("root.md");
    fs::write(&note_path, "# Root Note\n\n[[child]]").unwrap();

    let child_path = temp_dir.path().join("child.md");
    fs::write(&child_path, "# Child Note").unwrap();

    let params = create_initialize_params(Url::from_file_path(temp_dir.path()).unwrap());

    // Call initialize handler
    let result = handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    assert!(result.capabilities.completion_provider.is_some());

    // Check if engine was initialized in state
    let engine_lock = state.engine.read().await;
    assert!(engine_lock.is_some());
    let engine = engine_lock.as_ref().unwrap();
    let ws = &engine.workspace;

    assert!(ws.all_notes().len() >= 2);
}

#[tokio::test]
async fn test_lsp_completion() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let params = create_initialize_params(Url::from_file_path(temp_dir.path()).unwrap());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    let note_path = temp_dir.path().join("main.md");
    let content = "Check this: [[";
    fs::write(&note_path, content).unwrap();

    let uri = Url::from_file_path(&note_path).unwrap();

    handlers::handle_did_open(
        state,
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

    let response = handlers::handle_completion(client, state, completion_params)
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
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri);
    handlers::handle_initialize(client, state, params)
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
        state,
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
        state,
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

    let response = handlers::handle_goto_definition(client, state, definition_params)
        .await
        .unwrap();

    match response {
        Some(GotoDefinitionResponse::Scalar(location)) => {
            assert_eq!(location.uri, target_uri);
        }
        _ => panic!("Expected scalar location response to {:?}", target_uri),
    }
}

#[tokio::test]
async fn test_lsp_rename() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // 1. Create target note
    let _old_name = "old_note";
    let old_path = temp_dir.path().join("old_note.md");
    fs::write(&old_path, "# Old Note").unwrap();
    let old_uri = Url::from_file_path(&old_path).unwrap();

    // 2. Create source note with link
    let _source_name = "source";
    let source_path = temp_dir.path().join("source.md");
    let source_content = "Link to [[old_note]]";
    fs::write(&source_path, source_content).unwrap();
    let source_uri = Url::from_file_path(&source_path).unwrap();

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: old_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "# Old Note".to_string(),
            },
        },
    )
    .await;

    handlers::handle_did_open(
        state,
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

    // Verify "old_note" exists in workspace
    {
        let engine_lock = state.engine.read().await;
        let engine = engine_lock.as_ref().unwrap();
        let key_check = engine.workspace.resolve_note_key(&old_path);
        assert_eq!(
            key_check,
            Some("old_note".to_string()),
            "Key resolution failed"
        );
    }

    // 3. Request Rename: old_note -> new_note
    let new_name = "new_note";
    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: old_uri.clone(),
            },
            position: Position {
                line: 0,
                character: 0,
            },
        },
        new_name: new_name.to_string(),
        work_done_progress_params: Default::default(),
    };

    let response = handlers::rename::handle_rename(client, state, rename_params)
        .await
        .unwrap();

    assert!(response.is_some(), "Rename should return edits");
    let workspace_edit = response.unwrap();

    // Verify WorkspaceEdit
    let changes = workspace_edit.document_changes.unwrap();

    let mut rename_found = false;
    let mut link_update_found = false;

    match changes {
        DocumentChanges::Edits(_) => panic!("Expected DocumentChanges::Operations"),
        DocumentChanges::Operations(ops) => {
            for op in ops {
                match op {
                    DocumentChangeOperation::Op(ResourceOp::Rename(rename_file)) => {
                        if rename_file.old_uri == old_uri {
                            let expected_new_path = temp_dir.path().join("new_note.md");
                            let expected_new_uri = Url::from_file_path(expected_new_path).unwrap();
                            let actual_new_uri = rename_file.new_uri;

                            assert_eq!(actual_new_uri.path(), expected_new_uri.path());

                            rename_found = true;
                        }
                    }
                    DocumentChangeOperation::Edit(text_edit) => {
                        if text_edit.text_document.uri == source_uri {
                            assert!(!text_edit.edits.is_empty());
                            if let OneOf::Left(edit) = &text_edit.edits[0] {
                                assert_eq!(edit.new_text, "[[new_note]]");
                            } else {
                                panic!("Expected standard TextEdit");
                            }
                            link_update_found = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    assert!(rename_found, "Should find RenameFile operation");
    assert!(link_update_found, "Should find link update TextEdit");
}

#[tokio::test]
async fn test_lsp_multi_block_scenario() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // 1. Create Note B with multiple blocks
    let note_b_path = temp_dir.path().join("note_b.md");
    let note_b_content = "# Note B\n\nFirst block ^block-1\n\nSecond block ^block-2";
    fs::write(&note_b_path, note_b_content).unwrap();
    let note_b_uri = Url::from_file_path(&note_b_path).unwrap();

    // 2. Create Note A with links to Note B's blocks
    let note_a_path = temp_dir.path().join("note_a.md");
    let note_a_content = "# Note A\n\nLink 1: [[note_b#^block-1]]\nLink 2: [[note_b#^block-2]]";
    fs::write(&note_a_path, note_a_content).unwrap();
    let note_a_uri = Url::from_file_path(&note_a_path).unwrap();

    // Open both notes to populate workspace
    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_b_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: note_b_content.to_string(),
            },
        },
    )
    .await;

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_a_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: note_a_content.to_string(),
            },
        },
    )
    .await;

    // VERIFICATION 1: Hover
    // Hover on [[note_b#^block-1]]
    // Position: line 2, character 15ish
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: note_a_uri.clone(),
            },
            position: Position {
                line: 2,
                character: 15,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover = handlers::handle_hover(client, state, hover_params)
        .await
        .unwrap();
    assert!(hover.is_some());
    if let Hover {
        contents: HoverContents::Markup(markup),
        ..
    } = hover.unwrap()
    {
        assert!(markup.value.contains("First block"));
    } else {
        panic!("Expected markup hover contents");
    }

    // VERIFICATION 2: Goto Definition
    let definition_params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: note_a_uri.clone(),
            },
            position: Position {
                line: 3,
                character: 15, // [[note_b#^block-2]]
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let definition = handlers::handle_goto_definition(client, state, definition_params)
        .await
        .unwrap();
    match definition {
        Some(GotoDefinitionResponse::Scalar(location)) => {
            assert_eq!(location.uri, note_b_uri);
            // Should point to line 4 (where ^block-2 is)
            assert_eq!(location.range.start.line, 4);
        }
        _ => panic!("Expected definition to lead to Note B"),
    }

    // VERIFICATION 3: Rename Note B -> Note C
    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: note_b_uri.clone(),
            },
            position: Position {
                line: 0,
                character: 0,
            },
        },
        new_name: "note_c".to_string(),
        work_done_progress_params: Default::default(),
    };

    let rename_response = handlers::rename::handle_rename(client, state, rename_params)
        .await
        .unwrap();

    assert!(rename_response.is_some());
    let edits = rename_response.unwrap();

    if let Some(DocumentChanges::Operations(ops)) = edits.document_changes {
        let mut a_edits = Vec::new();
        for op in ops {
            if let DocumentChangeOperation::Edit(text_edit) = op {
                if text_edit.text_document.uri == note_a_uri {
                    a_edits.push(text_edit);
                }
            }
        }

        // Note A should have 2 edits for the 2 links
        assert_eq!(a_edits.len(), 1); // Grouped by file
        let edit_group = &a_edits[0];
        assert_eq!(edit_group.edits.len(), 2);

        if let OneOf::Left(edit) = &edit_group.edits[0] {
            assert_eq!(edit.new_text, "[[note_c#^block-1]]");
        }
        if let OneOf::Left(edit) = &edit_group.edits[1] {
            assert_eq!(edit.new_text, "[[note_c#^block-2]]");
        }
    } else {
        panic!("Expected DocumentChanges::Operations with edits");
    }
}

#[tokio::test]
async fn test_lsp_get_note_key() {
    let (backend, temp_dir) = setup_test_context().await;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(&backend.client, &backend.state, params)
        .await
        .unwrap();

    // 1. Create a note
    let note_path = temp_dir.path().join("my_note.md");
    let note_content = "# My Note";
    fs::write(&note_path, note_content).unwrap();
    let note_uri = Url::from_file_path(&note_path).unwrap();

    // Open note to populate workspace
    handlers::handle_did_open(
        &backend.state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: note_content.to_string(),
            },
        },
    )
    .await;

    // 2. Request Note Key
    let params = ExecuteCommandParams {
        command: "dendrite/getNoteKey".to_string(),
        arguments: vec![serde_json::to_value(crate::protocol::GetNoteKeyParams {
            uri: note_uri.to_string(),
        })
        .unwrap()],
        ..Default::default()
    };

    let response = backend.handle_execute_command(params).await.unwrap();

    assert!(response.is_some());
    let result: crate::protocol::GetNoteKeyResult =
        serde_json::from_value(response.unwrap()).unwrap();
    assert_eq!(result.key, "my_note");
}

#[tokio::test]
async fn test_lsp_rename_link_order_dendron() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // 1. Create Note B
    let note_b_path = temp_dir.path().join("note_b.md");
    let note_b_content = "# Note B";
    fs::write(&note_b_path, note_b_content).unwrap();
    let note_b_uri = Url::from_file_path(&note_b_path).unwrap();

    // 2. Create Note A with Dendron style link [[Alias|Target]]
    // According to user, Dendron is [[alias|target]]
    let note_a_path = temp_dir.path().join("note_a.md");
    let note_a_content = "# Note A\n\nLink: [[note_b_alias|note_b]]";
    fs::write(&note_a_path, note_a_content).unwrap();
    let note_a_uri = Url::from_file_path(&note_a_path).unwrap();

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_b_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: note_b_content.to_string(),
            },
        },
    )
    .await;

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_a_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: note_a_content.to_string(),
            },
        },
    )
    .await;

    // 3. Rename Note B -> Note C
    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: note_b_uri.clone(),
            },
            position: Position {
                line: 0,
                character: 0,
            },
        },
        new_name: "note_c".to_string(),
        work_done_progress_params: Default::default(),
    };

    let rename_response = handlers::rename::handle_rename(client, state, rename_params)
        .await
        .unwrap();

    assert!(rename_response.is_some());
    let edits = rename_response.unwrap();

    if let Some(DocumentChanges::Operations(ops)) = edits.document_changes {
        let mut a_edit_found = false;
        for op in ops {
            if let DocumentChangeOperation::Edit(text_edit) = op {
                if text_edit.text_document.uri == note_a_uri {
                    assert_eq!(text_edit.edits.len(), 1);
                    if let OneOf::Left(edit) = &text_edit.edits[0] {
                        // User says it SHOULD be [[note_b_alias|note_c]]
                        // If bug exists, it might be [[note_c|note_b_alias]]
                        assert_eq!(edit.new_text, "[[note_b_alias|note_c]]");
                    }
                    a_edit_found = true;
                }
            }
        }
        assert!(a_edit_found, "Note A should be updated");
    } else {
        panic!("Expected DocumentChanges::Operations with edits");
    }
}

#[tokio::test]
async fn test_lsp_code_action_split() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    let note_path = temp_dir.path().join("source.md");
    fs::write(&note_path, "Text to extract").unwrap();
    let note_uri = Url::from_file_path(&note_path).unwrap();

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "Text to extract".to_string(),
            },
        },
    )
    .await;

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier {
            uri: note_uri.clone(),
        },
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 4,
            },
        },
        context: CodeActionContext::default(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = backend.code_action(params).await.unwrap();
    assert!(result.is_some());
    let actions = result.unwrap();
    assert!(!actions.is_empty());

    // Find Refactor Code Action
    let split_action = actions.iter().find(|a| match a {
        CodeActionOrCommand::CodeAction(ca) => ca.title == "Refactor: Extract to New Note",
        _ => false,
    });

    assert!(
        split_action.is_some(),
        "Should offer Split Note code action"
    );
}

#[tokio::test]
async fn test_lsp_workspace_audit_command() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // Create a note with a broken link
    let note_path = temp_dir.path().join("broken.md");
    fs::write(&note_path, "Link to [[missing]]").unwrap();
    let note_uri = Url::from_file_path(&note_path).unwrap();

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "Link to [[missing]]".to_string(),
            },
        },
    )
    .await;

    // Trigger Audit
    let params = ExecuteCommandParams {
        command: "dendrite/workspaceAudit".to_string(),
        arguments: vec![],
        ..Default::default()
    };

    let result = backend.handle_execute_command(params).await.unwrap();

    // It should return the EditPlan as JSON
    assert!(result.is_some());
    let json = result.unwrap();

    // Structure should have "diagnostics"
    let diagnostics = json.get("diagnostics");
    assert!(
        diagnostics.is_some(),
        "Result should contain diagnostics field"
    );

    if let Some(arr) = diagnostics.and_then(|v| v.as_array()) {
        // We expect at least one error for broken link
        assert!(!arr.is_empty(), "Should report broken link");
        let msg = arr[0].get("message").and_then(|v| v.as_str()).unwrap();
        assert!(
            msg.contains("Broken link"),
            "Diagnostic should be about broken link"
        );
    } else {
        panic!("diagnostics should be an array");
    }
}

#[tokio::test]
async fn test_resolve_hierarchy_edits() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // 1. Create hierarchy: projects.active.one
    let note_path = temp_dir.path().join("projects.active.one.md");
    fs::write(&note_path, "# One").unwrap();
    let note_uri = Url::from_file_path(&note_path).unwrap();

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "# One".to_string(),
            },
        },
    )
    .await;

    // 2. Call dendrite/resolveHierarchyEdits
    // Old: projects.active -> New: archive.projects
    let params = ExecuteCommandParams {
        command: "dendrite/resolveHierarchyEdits".to_string(),
        arguments: vec![
            serde_json::to_value("projects.active").unwrap(),
            serde_json::to_value("archive.projects").unwrap(),
        ],
        ..Default::default()
    };

    let result = backend.handle_execute_command(params).await.unwrap();
    assert!(result.is_some());

    let moves: Vec<(String, String)> = serde_json::from_value(result.unwrap()).unwrap();

    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].0, "projects.active.one");
    assert_eq!(moves[0].1, "archive.projects.one");
}

#[tokio::test]
async fn test_lsp_get_backlinks() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // 1. Create target note
    let target_path = temp_dir.path().join("target.md");
    fs::write(&target_path, "# Target").unwrap();
    let target_uri = Url::from_file_path(&target_path).unwrap();

    // 2. Create source note
    let source_path = temp_dir.path().join("source.md");
    fs::write(&source_path, "Link to [[target]]").unwrap();
    let source_uri = Url::from_file_path(&source_path).unwrap();

    // Open to sync
    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: target_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "# Target".to_string(),
            },
        },
    )
    .await;

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: source_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "Link to [[target]]".to_string(),
            },
        },
    )
    .await;

    // 3. Call getBacklinks
    let params = ExecuteCommandParams {
        command: "dendrite/getBacklinks".to_string(),
        arguments: vec![serde_json::to_value(crate::protocol::GetBacklinksParams {
            note_key: "target".to_string(),
        })
        .unwrap()],
        ..Default::default()
    };

    let result = backend.handle_execute_command(params).await.unwrap();
    assert!(result.is_some());

    let backlinks_result: crate::protocol::GetBacklinksResult =
        serde_json::from_value(result.unwrap()).unwrap();

    assert_eq!(backlinks_result.backlinks.len(), 1);
    assert_eq!(backlinks_result.backlinks[0].key, "source");
}

#[tokio::test]
async fn test_delete_note_plan() {
    let (backend, temp_dir) = setup_test_context().await;
    let client = &backend.client;
    let state = &backend.state;

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let params = create_initialize_params(root_uri.clone());
    handlers::handle_initialize(client, state, params)
        .await
        .unwrap();

    // 1. Create note
    let note_path = temp_dir.path().join("todelete.md");
    fs::write(&note_path, "# Delete Me").unwrap();
    let note_uri = Url::from_file_path(&note_path).unwrap();

    handlers::handle_did_open(
        state,
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: note_uri.clone(),
                language_id: "markdown".to_string(),
                version: 0,
                text: "# Delete Me".to_string(),
            },
        },
    )
    .await;

    // 2. Get DendriteEngine and call delete_note
    {
        let engine_guard = state.engine.read().await;
        let engine = engine_guard.as_ref().unwrap();

        let plan = engine
            .delete_note("todelete")
            .expect("Should generate delete plan");

        assert_eq!(
            plan.mutation_kind,
            dendrite_core::mutation::model::MutationKind::DeleteNote
        );
        assert!(!plan.edits.is_empty());

        let edit_group = &plan.edits[0];
        // Check that the URI ends with the filename
        assert!(edit_group.uri.ends_with("todelete.md"));

        if let dendrite_core::mutation::model::Change::ResourceOp(
            dendrite_core::mutation::model::ResourceOperation::DeleteFile { .. },
        ) = &edit_group.changes[0]
        {
            // OK
        } else {
            panic!("Expected DeleteFile operation");
        }
    }
}
