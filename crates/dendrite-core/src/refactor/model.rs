use crate::model::TextRange;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPlan {
    pub refactor_kind: RefactorKind,
    pub edits: Vec<EditGroup>,
    pub preconditions: Vec<Precondition>,
    pub diagnostics: Vec<Diagnostic>,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactorKind {
    RenameNote,
    MoveNote,
    SplitNote,
    MergeNotes,
    UpdateLinks,
    ReorganizeHierarchy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditGroup {
    pub uri: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    TextEdit(TextEdit),
    ResourceOp(ResourceOperation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
    pub undo_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceOperation {
    CreateFile { content: Option<String> },
    DeleteFile { ignore_if_not_exists: bool },
    RenameFile { new_uri: String, overwrite: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Precondition {
    #[allow(private_interfaces)]
    NoteExists(String),
    PathNotExists(PathBuf),
    ContentUnchanged(PathBuf, String), // path, checksum
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}
