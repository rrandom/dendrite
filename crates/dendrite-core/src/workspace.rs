use std::path::PathBuf;
use walkdir::WalkDir;

use crate::hierarchy::HierarchyResolver;
use crate::identity::IdentityRegistry;
use crate::model::{Link, Note, NoteId, NoteKey};
use crate::parser::parse_markdown;
use crate::store::Store;

pub struct Workspace {
    root: PathBuf,
    resolver: Box<dyn HierarchyResolver>,
    identity: Box<dyn IdentityRegistry>,
    store: Store,
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
        }
    }

    pub fn on_file_open(&mut self, path: PathBuf, text: String) {
        self.update_file(&path, &text);
    }

    pub fn on_file_changed(&mut self, path: PathBuf, new_text: String) {
        self.update_file(&path, &new_text);
    }

    pub fn on_file_rename(&mut self, old_path: PathBuf, new_path: PathBuf, content: &str) {
        let Some(old_id) = self.store.note_id_by_path(&old_path).cloned() else {
            self.update_file(&new_path, content);
            return;
        };

        let old_key = self.identity.key_of(&old_id)
            .map(|(_, key)| key)
            .unwrap_or_else(|| self.resolver.note_key_from_path(&old_path, content));
        
        let new_key = self.resolver.note_key_from_path(&new_path, content);

        if old_key != new_key {
            let _ = self.identity.rebind(&old_key, &new_key);
        }

        let note = self.parse_note(content, &new_path, &old_id);
        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.store.upsert_note(note);
        self.store.bind_path(new_path, old_id.clone());
        self.store.set_outgoing_links(&old_id, targets);
    }

    pub fn on_file_delete(&mut self, path: PathBuf) {
        let Some(id) = self.store.note_id_by_path(&path).cloned() else {
            return;
        };
        self.store.remove_note(&id);
    }

    pub fn note_by_path(&self, path: &PathBuf) -> Option<&Note> {
        let id = self.store.note_id_by_path(path)?;
        self.store.get_note(id)
    }

    pub fn backlinks_of(&self, path: &PathBuf) -> Vec<PathBuf> {
        let Some(id) = self.store.note_id_by_path(&path) else {
            return vec![];
        };

        self.store
            .backlinks_of(&id)
            .iter()
            .filter_map(|backlink_id| {
                self.store
                    .get_note(backlink_id)
                    .and_then(|note| note.path.clone())
            })
            .collect()
    }

    pub fn all_notes(&self) -> Vec<&Note> {
        self.store.all_notes().collect()
    }

    /// Rename a note (semantic rename)
    pub fn rename_note(&mut self, old_path: PathBuf, new_key: NoteKey) {
        let old_key = self.resolver.note_key_from_path(&old_path, "");

        let Some(id) = self.identity.rebind(&old_key, &new_key) else {
            return;
        };

        let new_path = self.resolver.path_from_note_key(&new_key);
        self.store.update_path(&id, new_path);
    }

    /// Move a note to a new path
    pub fn move_note(&mut self, old_path: PathBuf, new_path: PathBuf) {
        let Some(id) = self.store.note_id_by_path(&old_path).cloned() else {
            let Ok(content) = std::fs::read_to_string(&new_path) else {
                return;
            };
            self.update_file(&new_path, &content);
            return;
        };

        let Ok(content) = std::fs::read_to_string(&new_path) else {
            return;
        };

        let Some((_, old_key)) = self.identity.key_of(&id) else {
            self.update_file(&new_path, &content);
            return;
        };

        let new_key = self.resolver.note_key_from_path(&new_path, &content);

        if old_key != new_key {
            let _ = self.identity.rebind(&old_key, &new_key);
        }

        let note = self.parse_note(&content, &new_path, &id);
        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.store.upsert_note(note);
        self.store.bind_path(new_path, id.clone());
        self.store.set_outgoing_links(&id, targets);
    }

    pub fn initialize(&mut self) -> Vec<PathBuf> {
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
        self.index_files(md_files.clone());

        md_files
    }

    pub fn update_file(&mut self, file_path: &PathBuf, content: &str) {
        let new_key = self.resolver.note_key_from_path(file_path, content);
        
        let note_id = if let Some(existing_id) = self.store.note_id_by_path(file_path) {
            let existing_id = existing_id.clone();
            
            if let Some((_, old_key)) = self.identity.key_of(&existing_id) {
                if old_key != new_key {
                    let _ = self.identity.rebind(&old_key, &new_key);
                }
            }
            
            existing_id
        } else {
            self.identity.get_or_create(&new_key)
        };
        
        let note = self.parse_note(content, file_path, &note_id);
        let targets: Vec<NoteId> = note.links.iter().map(|link| link.target.clone()).collect();
        self.store.upsert_note(note);
        self.store.bind_path(file_path.clone(), note_id.clone());
        self.store.set_outgoing_links(&note_id, targets);
    }

    pub fn index_files(&mut self, files: Vec<PathBuf>) {
        for path in files {
            self.index_file(path);
        }
    }

    fn index_file(&mut self, path: PathBuf) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        self.update_file(&path, &content);
    }

    fn parse_note(&mut self, content: &str, path: &PathBuf, note_id: &NoteId) -> Note {
        let parse_result = parse_markdown(content);
        let source_key = self.resolver.note_key_from_path(path, content);
        
        Note {
            id: note_id.clone(),
            path: Some(path.clone()),
            title: parse_result.title,
            frontmatter: parse_result.frontmatter,
            links: parse_result
                .links
                .iter()
                .map(|link| {
                    let link_key = self.resolver.note_key_from_link(&source_key, &link.target);
                    Link {
                        target: self.identity.get_or_create(&link_key),
                        range: link.range,
                        kind: link.kind.clone(),
                    }
                })
                .collect(),
            headings: parse_result.headings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::DendronStrategy;
    use crate::identity::DendriteIdentityRegistry;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> (Workspace, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let resolver = Box::new(DendronStrategy::new());
        let identity = Box::new(DendriteIdentityRegistry::new());
        let workspace = Workspace::new(temp_dir.path().to_path_buf(), resolver, identity);
        (workspace, temp_dir)
    }

    #[test]
    fn test_parse_note_resolves_links_correctly() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let note2_path = temp_dir.path().join("note2.md");
        fs::write(&note2_path, "# Note 2").unwrap();
        ws.on_file_open(note2_path.clone(), "# Note 2".to_string());
        let note2_id = ws.store.note_id_by_path(&note2_path).unwrap().clone();
        
        let note1_path = temp_dir.path().join("note1.md");
        let note1_content = "# Note 1\n\n[[note2]]";
        let note1_key = ws.resolver.note_key_from_path(&note1_path, note1_content);
        let note1_id = ws.identity.get_or_create(&note1_key);
        
        let note = ws.parse_note(note1_content, &note1_path, &note1_id);
        
        assert_eq!(note.links.len(), 1, "Should have one link");
        
        let link_target_key = ws.resolver.note_key_from_link(&note1_key, "note2");
        let note2_key = ws.identity.key_of(&note2_id).map(|(_, k)| k);
        
        assert_eq!(
            note.links[0].target, 
            note2_id,
            "Link target should point to note2's NoteId. Expected: {:?}, Got: {:?}, Link key: '{}', Note2 key: '{:?}'",
            note2_id,
            note.links[0].target,
            link_target_key,
            note2_key
        );
        
        assert_eq!(
            link_target_key,
            note2_key.unwrap_or_default(),
            "Link target key should match note2's key"
        );
    }

    #[test]
    fn test_note_id_stable_on_file_rename() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let file1_path = temp_dir.path().join("note1.md");
        fs::write(&file1_path, "# Note 1\n\nContent").unwrap();
        
        ws.on_file_open(file1_path.clone(), "# Note 1\n\nContent".to_string());
        let initial_id = ws.store.note_id_by_path(&file1_path).unwrap().clone();
        
        let file2_path = temp_dir.path().join("note2.md");
        fs::write(&file2_path, "# Note 1\n\nContent").unwrap();
        ws.on_file_rename(file1_path.clone(), file2_path.clone(), "# Note 1\n\nContent");
        
        let renamed_id = ws.store.note_id_by_path(&file2_path).unwrap();
        assert_eq!(&initial_id, renamed_id, "NoteId should remain stable after file rename");
    }

    #[test]
    fn test_note_id_stable_on_file_content_change() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let file_path = temp_dir.path().join("note.md");
        fs::write(&file_path, "# Note\n\nInitial content").unwrap();
        
        ws.on_file_open(file_path.clone(), "# Note\n\nInitial content".to_string());
        let initial_id = ws.store.note_id_by_path(&file_path).unwrap().clone();
        
        ws.on_file_changed(file_path.clone(), "# Note\n\nModified content".to_string());
        
        let changed_id = ws.store.note_id_by_path(&file_path).unwrap();
        assert_eq!(&initial_id, changed_id, "NoteId should remain stable after content change");
    }

    #[test]
    fn test_note_id_stable_on_file_move() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let file1_path = temp_dir.path().join("note1.md");
        fs::write(&file1_path, "# Note 1\n\nContent").unwrap();
        
        ws.on_file_open(file1_path.clone(), "# Note 1\n\nContent".to_string());
        let initial_id = ws.store.note_id_by_path(&file1_path).unwrap().clone();
        
        let file2_path = temp_dir.path().join("subdir").join("note1.md");
        fs::create_dir_all(file2_path.parent().unwrap()).unwrap();
        fs::write(&file2_path, "# Note 1\n\nContent").unwrap();
        
        ws.move_note(file1_path.clone(), file2_path.clone());
        
        let moved_id = ws.store.note_id_by_path(&file2_path).unwrap();
        assert_eq!(&initial_id, moved_id, "NoteId should remain stable after file move");
    }

    #[test]
    fn test_new_file_creates_new_note_id() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let file1_path = temp_dir.path().join("note1.md");
        fs::write(&file1_path, "# Note 1").unwrap();
        ws.on_file_open(file1_path.clone(), "# Note 1".to_string());
        let id1 = ws.store.note_id_by_path(&file1_path).unwrap().clone();
        
        let file2_path = temp_dir.path().join("note2.md");
        fs::write(&file2_path, "# Note 2").unwrap();
        ws.on_file_open(file2_path.clone(), "# Note 2".to_string());
        let id2 = ws.store.note_id_by_path(&file2_path).unwrap().clone();
        
        assert_ne!(id1, id2, "Different files should have different NoteIds");
    }

    #[test]
    fn test_backlinks_maintained_after_rename() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let note1_path = temp_dir.path().join("note1.md");
        let note2_path = temp_dir.path().join("note2.md");
        
        fs::write(&note1_path, "# Note 1\n\n[[note2]]").unwrap();
        fs::write(&note2_path, "# Note 2").unwrap();
        
        ws.on_file_open(note2_path.clone(), "# Note 2".to_string());
        let note2_id = ws.store.note_id_by_path(&note2_path).unwrap().clone();
        
        ws.on_file_open(note1_path.clone(), "# Note 1\n\n[[note2]]".to_string());
        
        let backlinks_before = ws.backlinks_of(&note2_path);
        assert!(backlinks_before.contains(&note1_path), "Note2 should have backlink from note1");
        
        let note2_new_path = temp_dir.path().join("note2_renamed.md");
        fs::write(&note2_new_path, "# Note 2").unwrap();
        ws.on_file_rename(note2_path.clone(), note2_new_path.clone(), "# Note 2");
        
        let note2_new_id = ws.store.note_id_by_path(&note2_new_path).unwrap();
        assert_eq!(&note2_id, note2_new_id, "NoteId should remain stable after rename");
        
        let backlinks_after = ws.backlinks_of(&note2_new_path);
        assert!(backlinks_after.contains(&note1_path), "Note2 should still have backlink from note1 after rename");
    }

    #[test]
    fn test_links_updated_after_content_change() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let note_path = temp_dir.path().join("note.md");
        fs::write(&note_path, "# Note\n\n[[target1]]").unwrap();
        
        ws.on_file_open(note_path.clone(), "# Note\n\n[[target1]]".to_string());
        let note = ws.note_by_path(&note_path).unwrap();
        let initial_id = note.id.clone();
        let _initial_link_count = note.links.len();
        
        ws.on_file_changed(note_path.clone(), "# Note\n\n[[target2]]".to_string());
        let updated_note = ws.note_by_path(&note_path).unwrap();
        
        assert_eq!(updated_note.links.len(), 1, "Should have one link");
        assert_eq!(initial_id, updated_note.id, "NoteId should remain stable");
    }

    #[test]
    fn test_semantic_rename_preserves_note_id() {
        let (mut ws, temp_dir) = create_test_workspace();
        
        let old_path = temp_dir.path().join("old_name.md");
        fs::write(&old_path, "# Old Name").unwrap();
        
        ws.on_file_open(old_path.clone(), "# Old Name".to_string());
        let initial_id = ws.store.note_id_by_path(&old_path).unwrap().clone();
        
        ws.rename_note(old_path.clone(), "new_name".to_string());
        
        let (_, new_key) = ws.identity.key_of(&initial_id).unwrap();
        assert_eq!(new_key, "new_name", "NoteKey should be updated");
        let new_id = ws.identity.lookup(&new_key).unwrap();
        assert_eq!(initial_id, new_id, "NoteId should remain stable after semantic rename");
    }
}
