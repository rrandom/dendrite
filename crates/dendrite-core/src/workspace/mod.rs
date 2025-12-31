use std::path::PathBuf;
use std::sync::RwLock;

use crate::hierarchy::HierarchyResolver;
use crate::identity::IdentityRegistry;
use crate::store::Store;

use walkdir::WalkDir;

mod file_ops;
mod hierarchy;
mod queries;

#[cfg(test)]
mod tests;

use hierarchy::NoteTree;

pub struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) resolver: Box<dyn HierarchyResolver>,
    pub(crate) identity: Box<dyn IdentityRegistry>,
    pub(crate) store: Store,
    pub(crate) tree_cache: RwLock<Option<NoteTree>>,
}

impl Workspace {
    pub fn new(
        root: PathBuf,
        resolver: Box<dyn HierarchyResolver>,
        identity: Box<dyn IdentityRegistry>,
    ) -> Self {
        Self {
            root,
            resolver,
            identity,
            store: Store::new(),
            tree_cache: RwLock::new(None),
        }
    }

    pub fn initialize(&mut self) -> Vec<PathBuf> {
        // Step 1: scan
        let mut md_files = Vec::new();

        for entry in WalkDir::new(&self.root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        md_files.push(path.to_path_buf());
                    }
                }
            }
        }

        // NoteId ↔ NoteKey
        self.index_files(md_files.clone());

        // build virtual notes
        self.fill_missing_hierarchy_levels();

        // build NoteTree
        self.invalidate_tree();

        md_files
    }
}
