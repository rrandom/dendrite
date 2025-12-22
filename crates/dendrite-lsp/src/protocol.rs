use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ListNotesParams {
    /// Filter query, corresponds to VS Code QuickPick input
    pub query: Option<String>,
}

/// NoteSummary matching api-contract.md
/// Must not contain UI fields like label, icon, has_children
#[derive(Debug, Serialize, Deserialize)]
pub struct NoteSummary {
    /// Unique note ID
    pub id: String,
    
    /// File URI (None if Ghost Node)
    pub uri: Option<String>,
    
    /// Title (for more friendly display)
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListNotesResult {
    pub notes: Vec<NoteSummary>,
}