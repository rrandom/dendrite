use crate::state::GlobalState;
use std::collections::HashMap;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

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
