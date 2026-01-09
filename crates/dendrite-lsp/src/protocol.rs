use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ListNotesParams {
    /// Filter query, corresponds to VS Code QuickPick input
    pub query: Option<String>,
}

/// NoteSummary matching api-contract.md
/// Must not contain UI fields like label, icon, has_children
#[derive(Debug, Serialize, Deserialize)]
pub struct NoteSummary {
    /// Note key
    pub key: String,

    /// File URI (None if Ghost Node)
    pub uri: Option<String>,

    /// Title (for more friendly display)
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListNotesResult {
    pub notes: Vec<NoteSummary>,
}

/// Parameters for dendrite/getHierarchy request
/// Currently empty, but can be extended in the future
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetHierarchyParams {}

/// Result for dendrite/getHierarchy request
/// Returns the complete hierarchy tree structure including Ghost Nodes
#[derive(Debug, Serialize, Deserialize)]
pub struct GetHierarchyResult {
    /// Root nodes of the hierarchy tree
    pub roots: Vec<dendrite_core::model::TreeView>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetNoteKeyParams {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetNoteKeyResult {
    pub key: String,
}
