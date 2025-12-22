use std::path::PathBuf;
use walkdir::WalkDir;

use crate::store::Store;
use crate::strategy::{BasicResolver, HierarchyResolver};

pub struct Workspace {
    /// Workspace root directory (absolute path)
    pub root: PathBuf,
    
    /// Data store
    pub store: Store,
    
    /// Strategy module (compatibility layer)
    /// Uses Box<dyn ...> for runtime polymorphism
    pub hierarchy_resolver: Box<dyn HierarchyResolver + Send + Sync>,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            store: Store::new(),
            hierarchy_resolver: Box::new(BasicResolver {}), 
        }
    }

    pub fn scan(&mut self) -> Vec<PathBuf> {
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
        
        md_files
    }

    /// Process file updates
    /// 1. Parse content
    /// 2. Resolver calculates ID and Parent
    /// 3. Store.insert
    pub fn update_file(&mut self, _abs_path: &PathBuf, _content: &str) {
        todo!()
    }
}