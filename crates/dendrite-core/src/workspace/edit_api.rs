use super::Workspace;
use crate::mutation::model::{Change, EditGroup, EditPlan, MutationKind, ResourceOperation};

impl Workspace {
    pub(crate) fn create_note(&self, note_key: &crate::model::NoteKey) -> Option<EditPlan> {
        let full_path = self.model.path_from_note_key(note_key);
        let uri = full_path.to_string_lossy().to_string();

        let content = self.model.generate_new_note_content(note_key);

        let edit_group = EditGroup {
            uri,
            changes: vec![Change::ResourceOp(ResourceOperation::CreateFile {
                content: Some(content),
            })],
        };

        Some(EditPlan {
            mutation_kind: MutationKind::CreateNote,
            edits: vec![edit_group],
            preconditions: vec![],
            diagnostics: vec![],
            reversible: true,
        })
    }
}
