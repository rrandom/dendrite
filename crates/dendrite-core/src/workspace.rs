use std::path::PathBuf;
use walkdir::WalkDir;

use crate::model::Note;
use crate::parser::parse_markdown;
use crate::store::Store;
use crate::strategy::{BasicResolver, HierarchyResolver};
use crate::normalize_path_to_id;

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
        
        // Parse and store all scanned files
        for file_path in &md_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                self.update_file(file_path, &content);
            }
        }
        
        md_files
    }

    /// Process file updates
    /// 1. Parse content
    /// 2. Resolver calculates ID and Parent
    /// 3. Store.insert
    pub fn update_file(&mut self, abs_path: &PathBuf, content: &str) {
        // 1. 计算 ID (Strategy)
        let relative_path = abs_path.strip_prefix(&self.root).ok();
        let note_id = if let Some(rel_path) = relative_path {
            self.hierarchy_resolver.resolve_id(&self.root, rel_path)
                .unwrap_or_else(|| normalize_path_to_id(rel_path))
        } else {
            normalize_path_to_id(abs_path)
        };

        // 2. 解析内容 (Parser)
        let parse_result = parse_markdown(content, &note_id);

        // 3. 构建 Note
        let note = Note {
            id: note_id.clone(),
            path: Some(abs_path.clone()),
            title: parse_result.title,
            frontmatter: parse_result.frontmatter,
            links: parse_result.links,
            headings: parse_result.headings,
        };

        // 4. 存入 Store
        self.store.upsert_note(note);
    }

    
    pub fn delete_file(&mut self, abs_path: &PathBuf) {
        // 1. 这里的难点是：我们只知道路径，得反查 ID
        // 这就是为什么 Store 里需要 path_map
        if let Some(note_id) = self.store.get_note_id_by_path(abs_path) {
            let id = note_id.clone(); // 避免借用冲突
            self.store.remove_note(&id);
        }
    }
}