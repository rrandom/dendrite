use super::Workspace;
use crate::vfs::FileSystem;
use std::path::PathBuf;
use std::sync::Arc;

/// The DendriteEngine acts as the high-level Facade for the Dendrite Core.
///
/// # Architecture Decision: Action vs Query Separation
///
/// *   **Actions (Write/Mutation)**: Unified in `DendriteEngine`.
///     All operations that modify state (File Sync) or calculate changes (Mutation)
///     SHOULD happen through methods on `DendriteEngine`. This ensures a single entry point for
///     business logic that may involve the FileSystem or Side Effects.
///
/// *   **Queries (Read)**: Access `engine.workspace` directly.
///     Read-only operations (resolving keys, looking up notes, graph traversal) DO NOT
///     need to be wrapped in `DendriteEngine`. Callers should access `engine.workspace` directly.
///     This avoids boilerplate and keeps the API surface clean.
pub struct DendriteEngine {
    pub workspace: Workspace,
    pub fs: Arc<dyn FileSystem>,
}

impl DendriteEngine {
    pub fn new(workspace: Workspace, fs: Arc<dyn FileSystem>) -> Self {
        Self { workspace, fs }
    }

    pub fn load_cache(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        use crate::cache::PersistentState;
        let state = PersistentState::load(path, &*self.fs)?;

        // Safety check: only load if model IDs match
        if state.model_id != self.workspace.model.id().0 {
            return Err("Cache model mismatch".into());
        }

        self.workspace.store = state.store;
        self.workspace.identity = state.identity;
        self.workspace.cache_metadata = state.metadata;
        self.workspace.invalidate_tree();
        Ok(())
    }

    pub fn save_cache(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        use crate::cache::PersistentState;
        let state = PersistentState {
            version: PersistentState::CURRENT_VERSION,
            model_id: self.workspace.model.id().0.to_string(),
            store: self.workspace.store.clone(),
            identity: self.workspace.identity.clone(),
            metadata: self.workspace.cache_metadata.clone(),
        };
        state.save(path, &*self.fs)
    }

    // ------------------------------------------------------------------------
    // File System Sync (Changes coming FROM disk)
    // ------------------------------------------------------------------------

    pub fn initialize(
        &mut self,
        _root: PathBuf,
    ) -> (Vec<PathBuf>, crate::workspace::indexer::IndexingStats) {
        self.workspace.initialize(&*self.fs)
    }

    pub fn update_content(&mut self, path: PathBuf, content: &str) {
        let vault_name = self
            .workspace
            .vault_name_for_path(&path)
            .unwrap_or_else(|| "main".to_string());
        self.workspace
            .update_file(path, content, vault_name, &*self.fs);
    }

    pub fn delete_file(&mut self, path: &PathBuf) {
        self.workspace.delete_file(path, &*self.fs);
    }

    pub fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        let vault_name = self
            .workspace
            .vault_name_for_path(&new_path)
            .unwrap_or_else(|| "main".to_string());
        self.workspace
            .rename_file(old_path, new_path, content, vault_name, &*self.fs);
    }

    // ------------------------------------------------------------------------
    // Mutation (Changes GOING TO disk)
    // ------------------------------------------------------------------------

    pub fn rename_note(
        &self,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::mutation::model::EditPlan> {
        self.workspace.rename_note(self, old_key, new_key)
    }

    pub fn move_note(
        &self,
        old_path: &std::path::Path,
        new_path: std::path::PathBuf,
    ) -> Option<crate::mutation::model::EditPlan> {
        self.workspace.move_note(self, old_path, new_path)
    }

    pub fn rename_hierarchy(
        &self,
        old_key: &str,
        new_key: &str,
    ) -> Option<crate::mutation::model::EditPlan> {
        self.workspace.rename_hierarchy(self, old_key, new_key)
    }

    pub fn split_note(
        &self,
        source_path: &std::path::Path,
        selection: crate::model::TextRange,
        new_note_title: &str,
    ) -> Option<crate::mutation::model::EditPlan> {
        self.workspace
            .split_note(self, source_path, selection, new_note_title)
    }

    // ------------------------------------------------------------------------
    // Note Editing & Health
    // ------------------------------------------------------------------------

    pub fn audit(&self) -> crate::mutation::model::EditPlan {
        self.workspace.audit()
    }

    pub fn create_note(
        &self,
        note_key: &crate::model::NoteKey,
    ) -> Option<crate::mutation::model::EditPlan> {
        self.workspace.create_note(note_key)
    }

    pub fn delete_note(&self, note_key: &str) -> Option<crate::mutation::model::EditPlan> {
        self.workspace.delete_note(note_key)
    }
}

impl crate::mutation::model::ContentProvider for DendriteEngine {
    fn get_content(&self, uri: &str) -> Option<String> {
        self.fs.read_to_string(&std::path::PathBuf::from(uri)).ok()
    }
}
