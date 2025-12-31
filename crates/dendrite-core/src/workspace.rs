use std::path::PathBuf;
use walkdir::WalkDir;

use crate::hierarchy::HierarchyResolver;
use crate::identity::IdentityRegistry;
use crate::model::{Link, Note, NoteId, NoteKey};
use crate::model::Point;
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

        let old_key = self
            .identity
            .key_of(&old_id)
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

    /// Find a link at the given position in a document
    pub fn find_link_at_position(&self, path: &PathBuf, position: Point) -> Option<&Link> {
        let note = self.note_by_path(path)?;
        note.links.iter().find(|link| {
            let range = link.range;
            // Check if position is within the link range
            (range.start.line < position.line
                || (range.start.line == position.line && range.start.col <= position.col))
                && (position.line < range.end.line
                    || (position.line == range.end.line && position.col <= range.end.col))
        })
    }

    /// Get the file path for a link's target
    pub fn get_link_target_path(&self, link: &Link) -> Option<PathBuf> {
        self.store.get_note(&link.target).and_then(|note| note.path.clone())
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
        ws.on_file_rename(
            file1_path.clone(),
            file2_path.clone(),
            "# Note 1\n\nContent",
        );

        let renamed_id = ws.store.note_id_by_path(&file2_path).unwrap();
        assert_eq!(
            &initial_id, renamed_id,
            "NoteId should remain stable after file rename"
        );
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
        assert_eq!(
            &initial_id, changed_id,
            "NoteId should remain stable after content change"
        );
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
        assert_eq!(
            &initial_id, moved_id,
            "NoteId should remain stable after file move"
        );
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
        assert!(
            backlinks_before.contains(&note1_path),
            "Note2 should have backlink from note1"
        );

        let note2_new_path = temp_dir.path().join("note2_renamed.md");
        fs::write(&note2_new_path, "# Note 2").unwrap();
        ws.on_file_rename(note2_path.clone(), note2_new_path.clone(), "# Note 2");

        let note2_new_id = ws.store.note_id_by_path(&note2_new_path).unwrap();
        assert_eq!(
            &note2_id, note2_new_id,
            "NoteId should remain stable after rename"
        );

        let backlinks_after = ws.backlinks_of(&note2_new_path);
        assert!(
            backlinks_after.contains(&note1_path),
            "Note2 should still have backlink from note1 after rename"
        );
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
        assert_eq!(
            initial_id, new_id,
            "NoteId should remain stable after semantic rename"
        );
    }

    #[test]
    fn test_find_link_at_position() {
        let (mut ws, temp_dir) = create_test_workspace();

        // Create target note
        let target_path = temp_dir.path().join("target.md");
        fs::write(&target_path, "# Target").unwrap();
        ws.on_file_open(target_path.clone(), "# Target".to_string());

        // Create source note with link
        let source_path = temp_dir.path().join("source.md");
        let source_content = "# Source\n\nThis is a link: [[target]]";
        fs::write(&source_path, source_content).unwrap();
        ws.on_file_open(source_path.clone(), source_content.to_string());

        // Get the note to check link position
        let note = ws.note_by_path(&source_path).unwrap();
        assert_eq!(note.links.len(), 1, "Should have one link");

        let link_range = note.links[0].range;
        
        // Test finding link at start position
        let link_at_start = ws.find_link_at_position(&source_path, link_range.start);
        assert!(link_at_start.is_some(), "Should find link at start position");
        assert_eq!(
            link_at_start.unwrap().target,
            note.links[0].target,
            "Found link should match"
        );

        // Test finding link at middle position
        let middle_point = Point {
            line: link_range.start.line,
            col: link_range.start.col + 2,
        };
        let link_at_middle = ws.find_link_at_position(&source_path, middle_point);
        assert!(link_at_middle.is_some(), "Should find link at middle position");

        // Test finding link at end position
        let link_at_end = ws.find_link_at_position(&source_path, link_range.end);
        assert!(link_at_end.is_some(), "Should find link at end position");
    }

    #[test]
    fn test_find_link_at_position_not_found() {
        let (mut ws, temp_dir) = create_test_workspace();

        // Create target note
        let target_path = temp_dir.path().join("target.md");
        fs::write(&target_path, "# Target").unwrap();
        ws.on_file_open(target_path.clone(), "# Target".to_string());

        // Create source note with link
        let source_path = temp_dir.path().join("source.md");
        let source_content = "# Source\n\nThis is a link: [[target]]";
        fs::write(&source_path, source_content).unwrap();
        ws.on_file_open(source_path.clone(), source_content.to_string());

        // Get the actual link range to calculate positions outside it
        let note = ws.note_by_path(&source_path).unwrap();
        assert_eq!(note.links.len(), 1, "Should have one link");
        let link_range = note.links[0].range;

        // Test finding link at position before link (before start)
        let before_link = Point {
            line: link_range.start.line,
            col: if link_range.start.col > 0 {
                link_range.start.col - 1
            } else {
                0
            },
        };
        let link_before = ws.find_link_at_position(&source_path, before_link);
        assert!(
            link_before.is_none(),
            "Should not find link before link position"
        );

        // Test finding link at position after link (after end)
        let after_link = Point {
            line: link_range.end.line,
            col: link_range.end.col + 1,
        };
        let link_after = ws.find_link_at_position(&source_path, after_link);
        assert!(
            link_after.is_none(),
            "Should not find link after link position"
        );

        // Test finding link at different line
        let different_line = Point { line: 0, col: 5 };
        let link_different_line = ws.find_link_at_position(&source_path, different_line);
        assert!(
            link_different_line.is_none(),
            "Should not find link at different line"
        );
    }

    #[test]
    fn test_find_link_at_position_multiple_links() {
        let (mut ws, temp_dir) = create_test_workspace();

        // Create target notes
        let target1_path = temp_dir.path().join("target1.md");
        let target2_path = temp_dir.path().join("target2.md");
        fs::write(&target1_path, "# Target 1").unwrap();
        fs::write(&target2_path, "# Target 2").unwrap();
        ws.on_file_open(target1_path.clone(), "# Target 1".to_string());
        ws.on_file_open(target2_path.clone(), "# Target 2".to_string());

        // Create source note with multiple links
        let source_path = temp_dir.path().join("source.md");
        let source_content = "# Source\n\n[[target1]] and [[target2]]";
        fs::write(&source_path, source_content).unwrap();
        ws.on_file_open(source_path.clone(), source_content.to_string());

        let note = ws.note_by_path(&source_path).unwrap();
        assert_eq!(note.links.len(), 2, "Should have two links");

        // Test finding first link
        let first_link_range = note.links[0].range;
        let first_link = ws.find_link_at_position(&source_path, first_link_range.start);
        assert!(first_link.is_some(), "Should find first link");
        assert_eq!(
            first_link.unwrap().target,
            note.links[0].target,
            "Found link should be first link"
        );

        // Test finding second link
        let second_link_range = note.links[1].range;
        let second_link = ws.find_link_at_position(&source_path, second_link_range.start);
        assert!(second_link.is_some(), "Should find second link");
        assert_eq!(
            second_link.unwrap().target,
            note.links[1].target,
            "Found link should be second link"
        );
    }

    #[test]
    fn test_get_link_target_path() {
        let (mut ws, temp_dir) = create_test_workspace();

        // Create target note
        let target_path = temp_dir.path().join("target.md");
        fs::write(&target_path, "# Target").unwrap();
        ws.on_file_open(target_path.clone(), "# Target".to_string());

        // Create source note with link
        let source_path = temp_dir.path().join("source.md");
        let source_content = "# Source\n\n[[target]]";
        fs::write(&source_path, source_content).unwrap();
        ws.on_file_open(source_path.clone(), source_content.to_string());

        // Get the link
        let note = ws.note_by_path(&source_path).unwrap();
        assert_eq!(note.links.len(), 1, "Should have one link");
        let link = &note.links[0];

        // Test getting target path
        let target_path_result = ws.get_link_target_path(link);
        assert!(
            target_path_result.is_some(),
            "Should get target path for existing link"
        );
        assert_eq!(
            target_path_result.unwrap(),
            target_path,
            "Target path should match"
        );
    }

    #[test]
    fn test_get_link_target_path_nonexistent() {
        let (mut ws, temp_dir) = create_test_workspace();

        // Create source note with link to non-existent target
        let source_path = temp_dir.path().join("source.md");
        let source_content = "# Source\n\n[[nonexistent]]";
        fs::write(&source_path, source_content).unwrap();
        ws.on_file_open(source_path.clone(), source_content.to_string());

        // Get the link
        let note = ws.note_by_path(&source_path).unwrap();
        assert_eq!(note.links.len(), 1, "Should have one link");
        let link = &note.links[0];

        // Test getting target path for non-existent target
        // Note: The link target will still have a NoteId (created on-the-fly),
        // but it won't have a path since the file doesn't exist
        let target_path_result = ws.get_link_target_path(link);
        // This should return None because the target note doesn't have a path
        assert!(
            target_path_result.is_none(),
            "Should return None for link to non-existent file"
        );
    }

    #[test]
    fn test_find_link_at_position_and_get_target_path_integration() {
        let (mut ws, temp_dir) = create_test_workspace();

        // Create target note
        let target_path = temp_dir.path().join("target.md");
        fs::write(&target_path, "# Target Note").unwrap();
        ws.on_file_open(target_path.clone(), "# Target Note".to_string());

        // Create source note with link
        let source_path = temp_dir.path().join("source.md");
        let source_content = "# Source Note\n\nCheck out [[target]] for more info.";
        fs::write(&source_path, source_content).unwrap();
        ws.on_file_open(source_path.clone(), source_content.to_string());

        // Find link at a position within the link
        // The link [[target]] should be on line 2 (0-based), around column 10-20
        let note = ws.note_by_path(&source_path).unwrap();
        let link_range = note.links[0].range;
        let position_in_link = Point {
            line: link_range.start.line,
            col: link_range.start.col + 3, // Position inside [[target]]
        };

        // Find the link
        let found_link = ws.find_link_at_position(&source_path, position_in_link);
        assert!(found_link.is_some(), "Should find link at position");

        // Get target path
        let target_path_result = ws.get_link_target_path(found_link.unwrap());
        assert!(
            target_path_result.is_some(),
            "Should get target path for found link"
        );
        assert_eq!(
            target_path_result.unwrap(),
            target_path,
            "Target path should match expected target"
        );
    }
}
