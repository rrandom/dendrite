use crate::state::GlobalState;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

pub async fn handle_did_change_configuration(
    client: &Client,
    state: &GlobalState,
    params: DidChangeConfigurationParams,
) {
    client
        .log_message(MessageType::INFO, "⚙️ Configuration changed")
        .await;

    // Typically, the settings are under a named section like "dendrite"
    // The structure depends on how the VS Code extension is configured.
    // We try to parse the 'settings' field if provided, or we could pull via workspace/configuration
    
    if let serde_json::Value::Object(map) = params.settings {
        if let Some(dendrite_settings) = map.get("dendrite") {
            match serde_json::from_value::<crate::config::LspSettings>(dendrite_settings.clone()) {
                Ok(new_settings) => {
                    let mut config_lock = state.config.write().await;
                    *config_lock = new_settings;
                    
                    // Respond to history limit change
                    let mut history = state.mutation_history.write().await;
                    while history.len() > config_lock.mutation_history_limit {
                        history.pop_front();
                    }
                    
                    client
                        .log_message(MessageType::INFO, "✅ LSP settings updated successfully")
                        .await;
                }
                Err(e) => {
                    client
                        .log_message(
                            MessageType::ERROR,
                            format!("❌ Failed to parse updated settings: {}", e),
                        )
                        .await;
                }
            }
        }
    }
}
