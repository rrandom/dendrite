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
    WorkspaceAudit,
    HierarchyRefactor,
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

    pub fn text_edit(self) -> Option<TextEdit> {
        match self {
            Change::TextEdit(t) => Some(t),
            _ => None,
        }
    }

    pub fn resource_op(self) -> Option<ResourceOperation> {
        match self {
            Change::ResourceOp(r) => Some(r),
            _ => None,
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
    pub uri: Option<String>,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;

    #[test]
    fn test_text_edit_invert() {
        let edit = TextEdit {
            range: TextRange {
                start: Point { line: 1, col: 5 },
                end: Point { line: 1, col: 10 },
            },
            new_text: "NewText\nMultiLine".to_string(),
            undo_text: Some("Old".to_string()),
        };

        let inverted = edit.invert();

        // The start should remain the same
        assert_eq!(inverted.range.start, Point { line: 1, col: 5 });
        // The end should accommodate the new text "NewText\nMultiLine"
        // "NewText" (7 chars) -> Point { line: 1, col: 12 }? No, wait.
        // My invert logic:
        // line 1, col 5
        // 'N', 'e', 'w', 'T', 'e', 'x', 't' -> col 12
        // '\n' -> line 2, col 0
        // 'M', 'u', 'l', 't', 'i', 'L', 'i', 'n', 'e' -> col 9
        assert_eq!(inverted.range.end, Point { line: 2, col: 9 });
        assert_eq!(inverted.new_text, "Old");
        assert_eq!(inverted.undo_text, Some("NewText\nMultiLine".to_string()));
    }

    #[test]
    fn test_resource_op_invert() {
        let op = ResourceOperation::RenameFile {
            new_uri: "new.md".to_string(),
            overwrite: false,
        };

        let inverted = op.invert("old.md");
        if let ResourceOperation::RenameFile { new_uri, .. } = inverted {
            assert_eq!(new_uri, "old.md");
        } else {
            panic!("Expected RenameFile");
        }
    }

    #[test]
    fn test_edit_group_invert() {
        let group = EditGroup {
            uri: "old.md".to_string(),
            changes: vec![Change::ResourceOp(ResourceOperation::RenameFile {
                new_uri: "new.md".to_string(),
                overwrite: false,
            })],
        };

        let inverted = group.invert();
        assert_eq!(inverted.uri, "new.md");
        if let Change::ResourceOp(ResourceOperation::RenameFile { new_uri, .. }) =
            &inverted.changes[0]
        {
            assert_eq!(new_uri, "old.md");
        } else {
            panic!("Expected RenameFile in inverted group");
        }
    }
}
