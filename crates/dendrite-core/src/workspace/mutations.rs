use super::Workspace;
use crate::mutation::model::{Change, EditGroup, EditPlan, MutationKind, ResourceOperation};

/// Refactoring Operations (Output)
/// These methods are triggered by user intent (LSP Commands).
/// They calculate changes (EditPlan) that SHOULD be applied to the file system.
/// They DO NOT modify files directly; they return a plan for the Client/LSP to execute.
impl Workspace {
    /// Initiate a standard Rename Refactoring from old_key to new_key.
    /// This ONLY renames the specific note, not its children.
    pub fn rename_note(
        &self,
        content_provider: &dyn crate::mutation::model::ContentProvider,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::mutation::model::EditPlan> {
        // 1. Lookup ID from Key
        let note_id = self.identity.lookup(&old_key.to_string())?;

        // 2. Calculate New Path (Forward Calculation using SemanticModel)
        let new_path = self.model.path_from_note_key(&new_key.to_string());

        // 3. Delegate to Core Refactor Engine (Structural only)
        crate::mutation::structural::calculate_structural_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.model.as_ref(),
            &note_id,
            new_path,
            new_key,
        )
    }

    /// Initiate a Hierarchy Refactoring.
    /// This renames the note AND all its descendants (e.g. `foo`->`bar` moves `foo.child`->`bar.child`).
    pub fn rename_hierarchy(
        &self,
        content_provider: &dyn crate::mutation::model::ContentProvider,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::mutation::model::EditPlan> {
        crate::mutation::hierarchy::calculate_hierarchy_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.model.as_ref(),
            old_key,
            new_key,
        )
    }

    /// Initiate a Move Refactoring from old_path to new_path.
    pub fn move_note(
        &self,
        content_provider: &dyn crate::mutation::model::ContentProvider,
        old_path: &std::path::Path,
        new_path: std::path::PathBuf,
    ) -> Option<crate::mutation::model::EditPlan> {
        // 1. Resolve ID from Old Path
        let note_id = self.store.note_id_by_path(&old_path.to_path_buf())?.clone();

        // 2. Resolve target Key from Target Path
        let new_key = self.model.note_key_from_path(&new_path, "");

        // 3. Delegate to Core Refactor Engine
        crate::mutation::structural::calculate_structural_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.model.as_ref(),
            &note_id,
            new_path,
            &new_key,
        )
    }

    /// Extract a selection into a new note (Split Note).
    pub fn split_note(
        &self,
        content_provider: &dyn crate::mutation::model::ContentProvider,
        source_path: &std::path::Path,
        selection: crate::model::TextRange,
        new_note_title: &str,
    ) -> Option<crate::mutation::model::EditPlan> {
        let source_id = self
            .store
            .note_id_by_path(&source_path.to_path_buf())?
            .clone();

        crate::mutation::split::calculate_split_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.model.as_ref(),
            &source_id,
            selection,
            new_note_title,
        )
    }
}

// Edit Operations (Output)
impl Workspace {
    pub fn create_note(&self, note_key: &crate::model::NoteKey) -> Option<EditPlan> {
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

    /// Delete a note.
    pub fn delete_note(&self, note_key: &str) -> Option<EditPlan> {
        let note_id = self.identity.lookup(&note_key.to_string())?;
        let note = self.store.get_note(&note_id)?;
        let path = note.path.as_ref()?;
        let uri = path.to_string_lossy().to_string();

        let edit_group = EditGroup {
            uri,
            changes: vec![Change::ResourceOp(ResourceOperation::DeleteFile {
                ignore_if_not_exists: false,
            })],
        };

        Some(EditPlan {
            mutation_kind: MutationKind::DeleteNote,
            edits: vec![edit_group],
            preconditions: vec![],
            diagnostics: vec![],
            reversible: true,
        })
    }
}
