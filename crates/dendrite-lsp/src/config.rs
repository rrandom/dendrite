use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspSettings {
    /// Logging level for the LSP server
    pub log_level: LogLevel,

    /// Persistent cache settings
    pub cache: CacheSettings,

    /// Maximum number of mutation history entries for undo
    pub mutation_history_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSettings {
    /// Whether to enable persistent cache (saving/loading)
    pub enabled: bool,

    /// Interval in milliseconds for debounced cache saving
    pub save_interval: u64,
}

impl Default for LspSettings {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            cache: CacheSettings {
                enabled: true,
                save_interval: 5000,
            },
            mutation_history_limit: 5,
        }
    }
}
