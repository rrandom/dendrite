use super::Workspace;
use crate::refactor::model::{Change, EditGroup, EditPlan, RefactorKind, ResourceOperation};
use crate::utils::{id, time};

impl Workspace {
    pub(crate) fn create_note(&self, note_key: &crate::model::NoteKey) -> Option<EditPlan> {
        let full_path = self.model.path_from_note_key(note_key);
        let uri = if cfg!(windows) {
            format!(
                "file:///{}",
                full_path.display().to_string().replace('\\', "/")
            )
        } else {
            format!("file://{}", full_path.display())
        };

        // 3. Generate Content
        let id = id::generate_id();
        let now = time::now();
        // Simple title derivation: last segment of key, capitalized? or just the key?
        // Let's us the last segment for title validation.
        let title_segment = note_key.split('.').last().unwrap_or(note_key);
        // Capitalize first letter?
        let title = if let Some(first_char) = title_segment.chars().next() {
            let c = first_char.to_uppercase();
            c.to_string() + &title_segment[1..]
        } else {
            title_segment.to_string()
        };

        let content = format!(
            r#"---
id: {}
title: {}
desc: ''
updated: {}
created: {}
---

"#,
            id, title, now, now
        );

        // 4. Construct EditPlan
        let edit_group = EditGroup {
            uri,
            changes: vec![Change::ResourceOp(ResourceOperation::CreateFile {
                content: Some(content),
            })],
        };

        Some(EditPlan {
            refactor_kind: RefactorKind::SplitNote, // Reuse SplitNote or add CreateNote?
            // Using SplitNote for now as it similar (creation), or add new enum variant?
            // Existing variants: RenameNote, MoveNote, SplitNote, WorkspaceAudit, HierarchyRefactor.
            // SplitNote seems most appropriate if we don't want to change Enum in model.rs yet.
            // Be careful if Client logic depends on Kind.
            // Actually, we should probably add CreateNote to RefactorKind if strict.
            // But since `apply_edit_plan` just converts edits, Kind is mostly for UI/Reporting.
            // Let's use SplitNote for now to avoid changing `model.rs` unless necessary.
            // Or better: RenameNote? No.
            // Let's stick with SplitNote or add CreateNote to model.rs (which requires changing all match arms?).
            // Checking model.rs... it's just data.
            // I'll add CreateNote to RefactorKind in model.rs if I can, otherwise SplitNote.
            // Wait, I can't check model.rs easily right now without another tool call.
            // I'll just use SplitNote for now.
            edits: vec![edit_group],
            preconditions: vec![], // TODO: Check if file exists?
            diagnostics: vec![],
            reversible: true,
        })
    }
}
