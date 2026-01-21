use super::assembler::NoteAssembler;
use super::Workspace;
use crate::model::NoteId;
use crate::parser::parse_markdown;
use crate::vfs::FileSystem;
use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct IndexingStats {
    pub total_files: usize,
    pub tier1_hits: usize,
    pub tier2_hits: usize,
    pub full_parses: usize,
}

/// Indexer responsible for orchestrating the indexing process.
/// It bridges I/O (FileSystem) and Workspace state.
pub struct Indexer<'a> {
    workspace: &'a mut Workspace,
    fs: &'a dyn FileSystem,
    stats: IndexingStats,
}

impl<'a> Indexer<'a> {
    pub fn new(workspace: &'a mut Workspace, fs: &'a dyn FileSystem) -> Self {
        Self {
            workspace,
            fs,
            stats: IndexingStats::default(),
        }
    }

    /// Performs a full index of the workspace using all configured vaults.
    pub fn full_index(&mut self) -> (Vec<PathBuf>, IndexingStats) {
        let extensions: Vec<String> = self
            .workspace
            .model
            .supported_extensions()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let vaults = self.workspace.config.workspace.vaults.clone();
        let mut all_files = Vec::new();

        for vault in vaults {
            let mut vault_files = Vec::new();
            for ext in &extensions {
                vault_files.extend(self.fs.list_files(&vault.path, ext));
            }

            for path in &vault_files {
                self.index_file(path.clone(), &vault.name);
            }
            all_files.extend(vault_files);
        }

        self.stats.total_files = all_files.len();

        // Build virtual notes for missing hierarchy levels
        self.workspace.fill_missing_hierarchy_levels();

        // Invalidate tree to trigger rebuild on next access
        self.workspace.invalidate_tree();

        (all_files, self.stats.clone())
    }

    /// Indexes a single file from disk.
    pub fn index_file(&mut self, path: PathBuf, vault_name: &str) {
        // Tier 1: Metadata Check
        if let Ok(fs_meta) = self.fs.metadata(&path) {
            if let Some(cached_meta) = self.workspace.cache_metadata.get(&path) {
                if cached_meta.mtime == fs_meta.mtime && cached_meta.size == fs_meta.len {
                    // Check if note exists in store
                    if self.workspace.store.note_id_by_path(&path).is_some() {
                        self.stats.tier1_hits += 1;
                        return; // Tier 1 Match!
                    }
                }
            }
        }

        let Ok(content) = self.fs.read_to_string(&path) else {
            return;
        };

        let digest = crate::parser::compute_digest(&content);

        // Tier 2: Digest Check
        if let Ok(fs_meta) = self.fs.metadata(&path) {
            if let Some(cached_meta) = self.workspace.cache_metadata.get(&path) {
                if cached_meta.digest == digest {
                    // Update metadata to catch next run in Tier 1
                    self.workspace.cache_metadata.insert(
                        path.clone(),
                        crate::cache::FileMetadata {
                            mtime: fs_meta.mtime,
                            size: fs_meta.len,
                            digest: digest.clone(),
                        },
                    );

                    if self.workspace.store.note_id_by_path(&path).is_some() {
                        self.stats.tier2_hits += 1;
                        return; // Tier 2 Match!
                    }
                }
            }
        }

        self.update_content_internal(path, &content, digest, vault_name.to_string());
    }

    /// Updates or creates a note from provided content.
    pub fn update_content(&mut self, path: PathBuf, content: &str, vault_name: String) {
        let digest = crate::parser::compute_digest(content);
        self.update_content_internal(path, content, digest, vault_name);
    }

    fn update_content_internal(
        &mut self,
        path: PathBuf,
        content: &str,
        digest: String,
        vault_name: String,
    ) {
        self.stats.full_parses += 1;
        let new_key = self.workspace.model.note_key_from_path(&path, content);

        let (note_id, _old_digest) =
            if let Some(existing_id) = self.workspace.store.note_id_by_path(&path) {
                let existing_id = existing_id.clone();
                let old_digest = self
                    .workspace
                    .store
                    .get_note(&existing_id)
                    .and_then(|n| n.digest.clone());

                let old_key = self
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

        // Parse with provided digest
        let mut parse_result =
            parse_markdown(content, &self.workspace.model.supported_link_kinds());

        // Override digest with our calculated one (just in case)
        parse_result.digest = digest.clone();

        let note = NoteAssembler::new(&*self.workspace.model, &mut self.workspace.identity)
            .assemble(parse_result, &path, &note_id, vault_name);

        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.workspace.store.upsert_note(note);
        self.workspace
            .store
            .bind_path(path.clone(), note_id.clone());
        self.workspace.store.set_outgoing_links(&note_id, targets);

        // Update cache metadata
        if let Ok(fs_meta) = self.fs.metadata(&path) {
            self.workspace.cache_metadata.insert(
                path,
                crate::cache::FileMetadata {
                    mtime: fs_meta.mtime,
                    size: fs_meta.len,
                    digest,
                },
            );
        }

        self.workspace.invalidate_tree();
    }

    /// Handles file renaming.
    pub fn rename_file(
        &mut self,
        old_path: PathBuf,
        new_path: PathBuf,
        content: &str,
        vault_name: String,
    ) {
        let Some(old_id) = self.workspace.store.note_id_by_path(&old_path).cloned() else {
            self.update_content(new_path, content, vault_name);
            return;
        };

        let old_key = self
            .workspace
            .identity
            .key_of(&old_id)
            .unwrap_or_else(|| self.workspace.model.note_key_from_path(&old_path, content));

        let new_key = self.workspace.model.note_key_from_path(&new_path, content);

        if old_key != new_key {
            let _ = self.workspace.identity.rebind(&old_key, &new_key);
        }

        let parse_result = parse_markdown(content, &self.workspace.model.supported_link_kinds());
        let note = NoteAssembler::new(&*self.workspace.model, &mut self.workspace.identity)
            .assemble(parse_result, &new_path, &old_id, vault_name);

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
