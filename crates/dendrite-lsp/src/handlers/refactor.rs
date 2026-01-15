use crate::state::GlobalState;
use dendrite_core::model::TextRange;
use dendrite_core::refactor::model::EditPlan;
use std::collections::HashMap;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

pub async fn handle_code_action(
    _client: &Client,
    _state: &GlobalState,
    params: CodeActionParams,
) -> Result<Option<Vec<CodeActionOrCommand>>> {
    // Only support RefactorExtract for now
    let mut actions = Vec::new();

    // Check if there is a selection (non-empty range)
    let range = params.range;
    if range.start != range.end {
        // Offer "Extract to New Note"
        let split_action = CodeAction {
            title: "Refactor: Extract to New Note".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            command: Some(Command {
                title: "Extract to New Note".to_string(),
                command: "dendrite.splitNote".to_string(), // Client -> Server command
                arguments: Some(vec![
                    serde_json::to_value(params.text_document.uri).unwrap(),
                    serde_json::to_value(range).unwrap(),
                ]),
            }),
            ..Default::default()
        };
        actions.push(CodeActionOrCommand::CodeAction(split_action));
    }

    Ok(Some(actions))
}

pub async fn handle_split_note_command(
    client: &Client,
    state: &GlobalState,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    // Arguments: [source_uri, range, new_note_name]
    if params.arguments.len() < 3 {
        return Err(Error {
            code: ErrorCode::InvalidParams,
            message: "Missing arguments: [source_uri, range, new_note_name]".into(),
            data: None,
        });
    }

    let source_uri: Url = serde_json::from_value(params.arguments[0].clone())
        .map_err(|_| Error::invalid_params("Invalid source_uri"))?;
    let range: Range = serde_json::from_value(params.arguments[1].clone())
        .map_err(|_| Error::invalid_params("Invalid range"))?;
    let new_note_name: String = serde_json::from_value(params.arguments[2].clone())
        .map_err(|_| Error::invalid_params("Invalid new_note_name"))?;

    let vault_guard = state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    let source_path = source_uri
        .to_file_path()
        .map_err(|_| Error::internal_error())?;

    // Verify key exists
    let _old_key = vault
        .workspace
        .resolve_note_key(&source_path)
        .ok_or_else(|| Error {
            code: ErrorCode::InvalidParams,
            message: format!("Could not find note key for path: {:?}", source_path).into(),
            data: None,
        })?;

    // Convert Range to TextRange
    let text_range = TextRange {
        start: dendrite_core::model::Point {
            line: range.start.line,
            col: range.start.character,
        },
        end: dendrite_core::model::Point {
            line: range.end.line,
            col: range.end.character,
        },
    };

    let plan = vault.split_note(&source_path, text_range, &new_note_name);

    if let Some(plan) = plan {
        apply_edit_plan(client, plan).await?;
        Ok(Some(serde_json::Value::Bool(true)))
    } else {
        Ok(Some(serde_json::Value::Bool(false)))
    }
}

pub async fn handle_workspace_audit_command(
    client: &Client,
    state: &GlobalState,
    _params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>> {
    let vault_guard = state.vault.read().await;
    let vault = vault_guard.as_ref().ok_or_else(Error::internal_error)?;

    let report = vault.audit();

    // Group diagnostics by URI
    let mut diagnostics_map: HashMap<Url, Vec<Diagnostic>> = HashMap::new();

    // Get root path from resolver
    let root_path = Some(vault.workspace.root());

    for diag in &report.diagnostics {
        if let Some((url, lsp_diag)) =
            crate::conversion::core_diagnostic_to_lsp_diagnostic(diag.clone(), root_path)
        {
            diagnostics_map.entry(url).or_default().push(lsp_diag);
        }
    }

    // Publish diagnostics
    for (uri, diags) in diagnostics_map {
        client.publish_diagnostics(uri, diags, None).await;
    }

    let diagnostic_count = report.diagnostics.len();
    let msg = format!(
        "Workspace Audit complete. Found {} issues.",
        diagnostic_count
    );
    client.show_message(MessageType::INFO, msg).await;

    Ok(Some(serde_json::to_value(report).unwrap()))
}

/// Helper to apply EditPlan via WorkspaceEdit
pub(crate) async fn apply_edit_plan(client: &Client, plan: EditPlan) -> Result<()> {
    let workspace_edit = crate::conversion::edit_plan_to_workspace_edit(plan);

    client
        .apply_edit(workspace_edit)
        .await?
        .applied
        .then_some(())
        .ok_or_else(|| Error {
            code: ErrorCode::InternalError,
            message: "Client failed to apply workspace edit".into(),
            data: None,
        })
}
