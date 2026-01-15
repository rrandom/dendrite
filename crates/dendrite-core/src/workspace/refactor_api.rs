use super::Workspace;

/// Refactoring Operations (Output)
/// These methods are triggered by user intent (LSP Commands).
/// They calculate changes (EditPlan) that SHOULD be applied to the file system.
/// They DO NOT modify files directly; they return a plan for the Client/LSP to execute.
impl Workspace {
    /// Initiate a standard Rename Refactoring from old_key to new_key.
    /// This ONLY renames the specific note, not its children.
    pub fn rename_note(
        &self,
        content_provider: &dyn crate::refactor::model::ContentProvider,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        // 1. Lookup ID from Key
        let note_id = self.identity.lookup(&old_key.to_string())?;

        // 2. Calculate New Path (Forward Calculation using SemanticModel)
        let new_path = self.model.path_from_note_key(&new_key.to_string());

        // 3. Delegate to Core Refactor Engine (Structural only)
        crate::refactor::structural::calculate_structural_edits(
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
        content_provider: &dyn crate::refactor::model::ContentProvider,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        crate::refactor::hierarchy::calculate_hierarchy_edits(
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
        content_provider: &dyn crate::refactor::model::ContentProvider,
        old_path: &std::path::Path,
        new_path: std::path::PathBuf,
    ) -> Option<crate::refactor::model::EditPlan> {
        // 1. Resolve ID from Old Path
        let note_id = self.store.note_id_by_path(&old_path.to_path_buf())?.clone();

        // 2. Resolve target Key from Target Path
        let new_key = self.model.note_key_from_path(&new_path, "");

        // 3. Delegate to Core Refactor Engine
        crate::refactor::structural::calculate_structural_edits(
            &self.store,
            &self.identity,
            content_provider,
            self.model.as_ref(),
            &note_id,
            new_path,
            &new_key,
        )
    }

    /// Audit the entire workspace for reference graph health.
    pub fn audit(&self) -> crate::refactor::model::EditPlan {
        crate::refactor::audit::calculate_audit_diagnostics(&self.store, self.model.as_ref())
    }

    /// Extract a selection into a new note (Split Note).
    pub fn split_note(
        &self,
        content_provider: &dyn crate::refactor::model::ContentProvider,
        source_path: &std::path::Path,
        selection: crate::model::TextRange,
        new_note_title: &str,
    ) -> Option<crate::refactor::model::EditPlan> {
        let source_id = self
            .store
            .note_id_by_path(&source_path.to_path_buf())?
            .clone();

        crate::refactor::split::calculate_split_edits(
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
