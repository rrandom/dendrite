use crate::model::TextRange;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub trait ContentProvider {
    fn get_content(&self, uri: &str) -> Option<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPlan {
    pub refactor_kind: RefactorKind,
    pub edits: Vec<EditGroup>,
    pub preconditions: Vec<Precondition>,
    pub diagnostics: Vec<Diagnostic>,
    pub reversible: bool,
}

impl EditPlan {
    pub fn invert(self) -> Self {
        Self {
            refactor_kind: self.refactor_kind,
            edits: self.edits.into_iter().map(|e| e.invert()).collect(),
            // Preconditions are harder to invert cleanly without more context.
            // For now, we clear them or keep them as is?
            // Actually, we should probably clear them as they relate to the original state.
            preconditions: vec![],
            diagnostics: self.diagnostics,
            reversible: self.reversible,
        }
    }
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

impl EditGroup {
    pub fn invert(self) -> Self {
        let mut target_uri = self.uri.clone();

        // If this group contains a rename, the inverse operation starts from the NEW uri
        for change in &self.changes {
            if let Change::ResourceOp(ResourceOperation::RenameFile { new_uri, .. }) = change {
                target_uri = new_uri.clone();
            }
        }

        Self {
            uri: target_uri,
            changes: self
                .changes
                .into_iter()
                .map(|c| c.invert(&self.uri))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    TextEdit(TextEdit),
    ResourceOp(ResourceOperation),
}

impl Change {
    pub fn invert(self, original_uri: &str) -> Self {
        match self {
            Change::TextEdit(t) => Change::TextEdit(t.invert()),
            Change::ResourceOp(r) => Change::ResourceOp(r.invert(original_uri)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
    pub undo_text: Option<String>,
}

impl TextEdit {
    pub fn invert(self) -> Self {
        let old_text = self.undo_text.expect("Inversion requires undo_text");
        let new_text = self.new_text;

        // Calculate the range of 'new_text' which is what we will replace when undoing
        let mut end = self.range.start;
        for c in new_text.chars() {
            if c == '\n' {
                end.line += 1;
                end.col = 0;
            } else {
                end.col += 1;
            }
        }

        Self {
            range: crate::model::TextRange {
                start: self.range.start,
                end,
            },
            new_text: old_text,
            undo_text: Some(new_text),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceOperation {
    CreateFile { content: Option<String> },
    DeleteFile { ignore_if_not_exists: bool },
    RenameFile { new_uri: String, overwrite: bool },
}

impl ResourceOperation {
    pub fn invert(self, original_uri: &str) -> Self {
        match self {
            ResourceOperation::RenameFile { .. } => ResourceOperation::RenameFile {
                new_uri: original_uri.to_string(),
                overwrite: false,
            },
            _ => self, // TODO: Support Create/Delete inversion
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
