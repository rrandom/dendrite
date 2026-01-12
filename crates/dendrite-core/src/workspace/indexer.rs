use super::assembler::NoteAssembler;
use super::vfs::FileSystem;
use super::Workspace;
use crate::model::NoteId;
use crate::parser::parse_markdown;
use std::path::PathBuf;

/// Indexer responsible for orchestrating the indexing process.
/// It bridges I/O (FileSystem) and Workspace state.
pub struct Indexer<'a> {
    workspace: &'a mut Workspace,
    fs: &'a dyn FileSystem,
}

impl<'a> Indexer<'a> {
    pub fn new(workspace: &'a mut Workspace, fs: &'a dyn FileSystem) -> Self {
        Self { workspace, fs }
    }

    /// Performs a full index of the workspace.
    pub fn full_index(&mut self, root: PathBuf) -> Vec<PathBuf> {
        let files = self.fs.list_files(&root, "md");

        for path in &files {
            self.index_file(path.clone());
        }

        // Build virtual notes for missing hierarchy levels
        self.workspace.fill_missing_hierarchy_levels();

        // Invalidate tree to trigger rebuild on next access
        self.workspace.invalidate_tree();

        files
    }

    /// Indexes a single file from disk.
    pub fn index_file(&mut self, path: PathBuf) {
        let Ok(content) = self.fs.read_to_string(&path) else {
            return;
        };
        self.update_content(path, &content);
    }

    /// Updates or creates a note from provided content.
    pub fn update_content(&mut self, path: PathBuf, content: &str) {
        let new_key = self.workspace.resolver.note_key_from_path(&path, content);

        let (note_id, old_digest) =
            if let Some(existing_id) = self.workspace.store.note_id_by_path(&path) {
                let existing_id = existing_id.clone();
                let old_digest = self
                    .workspace
                    .store
                    .get_note(&existing_id)
                    .and_then(|n| n.digest.clone());

                let (_, old_key) = self
                    .workspace
                    .identity
                    .key_of(&existing_id)
                    .expect("Consistency error: note ID without key");
                if old_key != new_key {
                    let _ = self.workspace.identity.rebind(&old_key, &new_key);
                }

                (existing_id, old_digest)
            } else {
                (self.workspace.identity.get_or_create(&new_key), None)
            };

        // Parse always to get the new digest
        let parse_result = parse_markdown(content, self.workspace.resolver.wikilink_format());

        if let Some(old) = old_digest {
            if old == parse_result.digest {
                // Content unchanged, skip update
                return;
            }
        }

        let note = NoteAssembler::new(&*self.workspace.resolver, &mut self.workspace.identity)
            .assemble(parse_result, &path, &note_id);

        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.workspace.store.upsert_note(note);
        self.workspace.store.bind_path(path, note_id.clone());
        self.workspace.store.set_outgoing_links(&note_id, targets);

        self.workspace.invalidate_tree();
    }

    /// Handles file renaming.
    pub fn rename_file(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        let Some(old_id) = self.workspace.store.note_id_by_path(&old_path).cloned() else {
            self.update_content(new_path, content);
            return;
        };

        let old_key = self
            .workspace
            .identity
            .key_of(&old_id)
            .map(|(_, key)| key)
            .unwrap_or_else(|| {
                self.workspace
                    .resolver
                    .note_key_from_path(&old_path, content)
            });

        let new_key = self
            .workspace
            .resolver
            .note_key_from_path(&new_path, content);

        if old_key != new_key {
            let _ = self.workspace.identity.rebind(&old_key, &new_key);
        }

        let parse_result = parse_markdown(content, self.workspace.resolver.wikilink_format());
        let note = NoteAssembler::new(&*self.workspace.resolver, &mut self.workspace.identity)
            .assemble(parse_result, &new_path, &old_id);

        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.workspace.store.upsert_note(note);
        self.workspace.store.bind_path(new_path, old_id.clone());
        self.workspace.store.set_outgoing_links(&old_id, targets);

        // Key change affects tree structure
        if old_key != new_key {
            self.workspace.invalidate_tree();
        }
    }

    /// Handles file deletion.
    pub fn delete_file(&mut self, path: &PathBuf) {
        let Some(id) = self.workspace.store.note_id_by_path(path).cloned() else {
            return;
        };
        self.workspace.store.remove_note(&id);
        self.workspace.invalidate_tree();
    }
}
