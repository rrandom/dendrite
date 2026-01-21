use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration for Dendrite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DendriteConfig {
    pub workspace: WorkspaceConfig,
    pub semantic: SemanticConfig,
    pub logging: LoggingConfig,
}

/// Workspace-level settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Name of the workspace
    pub name: String,
    /// List of vaults in this workspace
    pub vaults: Vec<VaultConfig>,
    /// Global ignore patterns (glob syntax)
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

/// Individual vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Friendly name of the vault
    pub name: String,
    /// Physical path to the vault directory (relative to workspace root)
    pub path: PathBuf,
}

/// Semantic model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConfig {
    /// The model ID (e.g., "Dendron", "Obsidian")
    pub model: String,
    /// Model-specific parameters as a JSON object
    #[serde(default = "empty_json_value")]
    pub settings: serde_json::Value,
}

/// Logging and telemetery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Whether to show indexing statistics on startup
    #[serde(default = "default_true")]
    pub show_indexing_stats: bool,
}

fn empty_json_value() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn default_true() -> bool {
    true
}

impl Default for DendriteConfig {
    fn default() -> Self {
        Self {
            workspace: WorkspaceConfig {
                name: "Dendrite Workspace".to_string(),
                vaults: vec![VaultConfig {
                    name: "main".to_string(),
                    path: PathBuf::from("."),
                }],
                ignore_patterns: vec![
                    "**/.git/**".to_string(),
                    "**/node_modules/**".to_string(),
                ],
            },
            semantic: SemanticConfig {
                model: "Dendron".to_string(),
                settings: empty_json_value(),
            },
            logging: LoggingConfig {
                show_indexing_stats: true,
            },
        }
    }
}

impl DendriteConfig {
    /// Load config from a specific path
    pub fn from_yaml(content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(content)
    }

    /// Serialize to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}
