use super::*;
use crate::hierarchy::DendronStrategy;
use crate::identity::BasicIdentityRegistry;
use crate::model::Point;
use crate::parser::parse_markdown;
use std::fs;
use tempfile::TempDir;

fn create_test_workspace() -> (Workspace, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let resolver = Box::new(DendronStrategy::new(temp_dir.path().to_path_buf()));
    let identity = Box::new(BasicIdentityRegistry::new());
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

    let parse_result = parse_markdown(note1_content);
    let note = ws.create_note_from_parse(parse_result, &note1_path, &note1_id);

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
    assert!(
        link_at_start.is_some(),
        "Should find link at start position"
    );
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
    assert!(
        link_at_middle.is_some(),
        "Should find link at middle position"
    );

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

#[test]
fn test_all_note_keys() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create multiple notes with different titles
    let note1_path = temp_dir.path().join("note1.md");
    fs::write(&note1_path, "# Note One").unwrap();
    ws.on_file_open(note1_path.clone(), "# Note One".to_string());

    let note2_path = temp_dir.path().join("note2.md");
    fs::write(&note2_path, "# Note Two").unwrap();
    ws.on_file_open(note2_path.clone(), "# Note Two".to_string());

    let note3_path = temp_dir.path().join("note3.md");
    fs::write(&note3_path, "No title here").unwrap();
    ws.on_file_open(note3_path.clone(), "No title here".to_string());

    // Get all note keys
    let note_keys = ws.all_note_keys();

    // Should have 3 notes
    assert_eq!(note_keys.len(), 3, "Should have 3 notes");

    // Check that keys and display names are correct
    // Note: HashMap iteration order is not guaranteed, so we can't rely on index
    let keys: Vec<String> = note_keys.iter().map(|(k, _)| k.clone()).collect();
    let display_names: Vec<String> = note_keys.iter().map(|(_, d)| d.clone()).collect();

    // Note keys should be just the filename without .md extension (Dendron design)
    // e.g., "note1.md" -> "note1"
    assert!(
        keys.contains(&"note1".to_string()),
        "Should contain note1 key, got: {:?}",
        keys
    );
    assert!(
        keys.contains(&"note2".to_string()),
        "Should contain note2 key, got: {:?}",
        keys
    );
    assert!(
        keys.contains(&"note3".to_string()),
        "Should contain note3 key, got: {:?}",
        keys
    );

    // Display names should match titles
    assert!(
        display_names.contains(&"Note One".to_string()),
        "Should contain 'Note One' as display name"
    );
    assert!(
        display_names.contains(&"Note Two".to_string()),
        "Should contain 'Note Two' as display name"
    );
    // Note without title should have empty display name
    assert!(
        display_names.contains(&"".to_string()),
        "Should contain empty display name for note without title"
    );
}

#[test]
fn test_virtual_notes_created_for_missing_parents() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create a note with hierarchical key: "foo.bar.baz.md"
    // This should create virtual notes for "foo" and "foo.bar"
    let baz_path = temp_dir.path().join("foo.bar.baz.md");
    fs::write(&baz_path, "# Baz").unwrap();
    ws.on_file_open(baz_path.clone(), "# Baz".to_string());

    // Initialize workspace to trigger virtual note creation
    ws.initialize();

    // Check that virtual notes were created
    let all_notes: Vec<_> = ws.store.all_notes().collect();

    // Should have 3 notes: "foo.bar.baz" (real) + "foo" (virtual) + "foo.bar" (virtual)
    assert_eq!(
        all_notes.len(),
        3,
        "Should have 3 notes (1 real + 2 virtual)"
    );

    // Check that "foo" virtual note exists
    let foo_key = "foo".to_string();
    let foo_id = ws.identity.lookup(&foo_key);
    assert!(foo_id.is_some(), "Virtual note 'foo' should exist");
    let foo_note = ws.store.get_note(foo_id.as_ref().unwrap());
    assert!(foo_note.is_some(), "Virtual note 'foo' should be in store");
    assert!(
        foo_note.unwrap().path.is_none(),
        "Virtual note 'foo' should have no path"
    );

    // Check that "foo.bar" virtual note exists
    let foobar_key = "foo.bar".to_string();
    let foobar_id = ws.identity.lookup(&foobar_key);
    assert!(foobar_id.is_some(), "Virtual note 'foo.bar' should exist");
    let foobar_note = ws.store.get_note(foobar_id.as_ref().unwrap());
    assert!(
        foobar_note.is_some(),
        "Virtual note 'foo.bar' should be in store"
    );
    assert!(
        foobar_note.unwrap().path.is_none(),
        "Virtual note 'foo.bar' should have no path"
    );

    // Check that "foo.bar.baz" real note exists
    let baz_key = "foo.bar.baz".to_string();
    let baz_id = ws.identity.lookup(&baz_key);
    assert!(baz_id.is_some(), "Real note 'foo.bar.baz' should exist");
    let baz_note = ws.store.get_note(baz_id.as_ref().unwrap());
    assert!(
        baz_note.is_some(),
        "Real note 'foo.bar.baz' should be in store"
    );
    assert!(
        baz_note.unwrap().path.is_some(),
        "Real note 'foo.bar.baz' should have path"
    );
}

#[test]
fn test_tree_structure_built_correctly() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create hierarchical notes
    let foo_path = temp_dir.path().join("foo.md");
    fs::write(&foo_path, "# Foo").unwrap();
    ws.on_file_open(foo_path.clone(), "# Foo".to_string());

    let foobar_path = temp_dir.path().join("foo.bar.md");
    fs::write(&foobar_path, "# Foo Bar").unwrap();
    ws.on_file_open(foobar_path.clone(), "# Foo Bar".to_string());

    let foobarbaz_path = temp_dir.path().join("foo.bar.baz.md");
    fs::write(&foobarbaz_path, "# Foo Bar Baz").unwrap();
    ws.on_file_open(foobarbaz_path.clone(), "# Foo Bar Baz".to_string());

    // Initialize to build tree
    ws.initialize();

    // Get tree structure
    let tree = ws.tree();

    // "foo" should be a root node
    let foo_key = "foo".to_string();
    let foo_id = ws.identity.lookup(&foo_key).unwrap();
    assert!(
        tree.root_nodes.contains(&foo_id),
        "foo should be a root node"
    );

    // "foo.bar" should be a child of "foo"
    let foobar_key = "foo.bar".to_string();
    let foobar_id = ws.identity.lookup(&foobar_key).unwrap();
    assert!(
        tree.children
            .get(&foo_id)
            .map(|c| c.contains(&foobar_id))
            .unwrap_or(false),
        "foo.bar should be a child of foo"
    );

    // "foo.bar.baz" should be a child of "foo.bar"
    let foobarbaz_key = "foo.bar.baz".to_string();
    let foobarbaz_id = ws.identity.lookup(&foobarbaz_key).unwrap();
    assert!(
        tree.children
            .get(&foobar_id)
            .map(|c| c.contains(&foobarbaz_id))
            .unwrap_or(false),
        "foo.bar.baz should be a child of foo.bar"
    );

    // Check parent relationships
    assert_eq!(
        tree.parent.get(&foobar_id),
        Some(&foo_id),
        "foo.bar's parent should be foo"
    );
    assert_eq!(
        tree.parent.get(&foobarbaz_id),
        Some(&foobar_id),
        "foo.bar.baz's parent should be foo.bar"
    );
}

#[test]
fn test_tree_cache_works() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create a note
    let note_path = temp_dir.path().join("test.md");
    fs::write(&note_path, "# Test").unwrap();
    ws.on_file_open(note_path.clone(), "# Test".to_string());

    // Initialize to build tree
    ws.initialize();

    // First call should build the tree
    let tree1 = ws.tree();
    assert!(!tree1.root_nodes.is_empty(), "Tree should be built");

    // Second call should use cache (same tree structure)
    let tree2 = ws.tree();
    assert_eq!(
        tree1.root_nodes, tree2.root_nodes,
        "Cached tree should match"
    );
}

#[test]
fn test_tree_invalidated_on_file_changes() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create initial note
    let note_path = temp_dir.path().join("test.md");
    fs::write(&note_path, "# Test").unwrap();
    ws.on_file_open(note_path.clone(), "# Test".to_string());

    // Initialize to build tree
    ws.initialize();

    // Get initial tree
    let tree1 = ws.tree();
    let initial_root_count = tree1.root_nodes.len();

    // Add a new note (should invalidate tree)
    let note2_path = temp_dir.path().join("test2.md");
    fs::write(&note2_path, "# Test 2").unwrap();
    ws.on_file_open(note2_path.clone(), "# Test 2".to_string());

    // Tree should be rebuilt with new note
    let tree2 = ws.tree();
    assert!(
        tree2.root_nodes.len() > initial_root_count,
        "Tree should be rebuilt with new note"
    );

    // Delete a note (should invalidate tree)
    ws.on_file_delete(note_path.clone());

    // Tree should be rebuilt without deleted note
    let tree3 = ws.tree();
    assert!(
        tree3.root_nodes.len() < tree2.root_nodes.len(),
        "Tree should be rebuilt without deleted note"
    );
}

#[test]
fn test_get_tree_view() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create hierarchical notes
    let foo_path = temp_dir.path().join("foo.md");
    fs::write(&foo_path, "# Foo").unwrap();
    ws.on_file_open(foo_path.clone(), "# Foo".to_string());

    let foobar_path = temp_dir.path().join("foo.bar.md");
    fs::write(&foobar_path, "# Foo Bar").unwrap();
    ws.on_file_open(foobar_path.clone(), "# Foo Bar".to_string());

    // Initialize to build tree and create virtual notes
    ws.initialize();

    // Get tree view
    let tree_view = ws.get_tree_view();

    // Should have root nodes
    assert!(!tree_view.is_empty(), "Tree view should have root nodes");

    // Find "foo" node
    let foo_node = tree_view
        .iter()
        .find(|node| node.note.key.as_ref() == Some(&"foo".to_string()));
    assert!(foo_node.is_some(), "Should find 'foo' node in tree view");
    let foo_node = foo_node.unwrap();

    // Check that "foo" has children
    assert!(!foo_node.children.is_empty(), "foo should have children");

    // Find "foo.bar" in children
    let foobar_node = foo_node
        .children
        .iter()
        .find(|node| node.note.key.as_ref() == Some(&"foo.bar".to_string()));
    assert!(
        foobar_node.is_some(),
        "Should find 'foo.bar' as child of 'foo'"
    );

    // Check NoteRef structure
    let foobar_ref = &foobar_node.unwrap().note;
    assert_eq!(
        foobar_ref.key,
        Some("foo.bar".to_string()),
        "Key should match"
    );
    assert!(foobar_ref.path.is_some(), "Real note should have path");
    assert_eq!(
        foobar_ref.title,
        Some("Foo Bar".to_string()),
        "Title should match"
    );
}

#[test]
fn test_virtual_notes_in_tree_view() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create a note with hierarchical key (missing parents)
    let baz_path = temp_dir.path().join("foo.bar.baz.md");
    fs::write(&baz_path, "# Baz").unwrap();
    ws.on_file_open(baz_path.clone(), "# Baz".to_string());

    // Initialize to create virtual notes
    ws.initialize();

    // Get tree view
    let tree_view = ws.get_tree_view();

    // Find "foo" virtual node
    let foo_node = tree_view
        .iter()
        .find(|node| node.note.key.as_ref() == Some(&"foo".to_string()));
    assert!(foo_node.is_some(), "Should find 'foo' virtual node");
    let foo_node = foo_node.unwrap();

    // Virtual note should have no path
    assert!(
        foo_node.note.path.is_none(),
        "Virtual note should have no path"
    );
    assert!(
        foo_node.note.title.is_none(),
        "Virtual note should have no title"
    );

    // "foo" should have "foo.bar" as child
    let foobar_node = foo_node
        .children
        .iter()
        .find(|node| node.note.key.as_ref() == Some(&"foo.bar".to_string()));
    assert!(
        foobar_node.is_some(),
        "Should find 'foo.bar' as child of 'foo'"
    );
    let foobar_node = foobar_node.unwrap();

    // "foo.bar" should have "foo.bar.baz" as child
    let baz_node = foobar_node
        .children
        .iter()
        .find(|node| node.note.key.as_ref() == Some(&"foo.bar.baz".to_string()));
    assert!(
        baz_node.is_some(),
        "Should find 'foo.bar.baz' as child of 'foo.bar'"
    );
    let baz_node = baz_node.unwrap();

    // Real note should have path
    assert!(baz_node.note.path.is_some(), "Real note should have path");
    assert_eq!(
        baz_node.note.title,
        Some("Baz".to_string()),
        "Real note should have title"
    );
}

#[test]
fn test_resolve_link_anchor() {
    let (mut ws, temp_dir) = create_test_workspace();

    // Create target note with headings and blocks
    let target_path = temp_dir.path().join("target.md");
    let target_content = "# Heading 1\n\nSome text ^block-1\n\n## Heading 2\n\nMore text ^block-2";
    fs::write(&target_path, target_content).unwrap();
    ws.on_file_open(target_path.clone(), target_content.to_string());

    let target_id = ws.store.note_id_by_path(&target_path).unwrap().clone();

    // Create source note with links to anchors
    let source_path = temp_dir.path().join("source.md");
    let source_content = "[[target#Heading 1]], [[target#^block-2]]";
    fs::write(&source_path, source_content).unwrap();
    ws.on_file_open(source_path.clone(), source_content.to_string());

    let note = ws.note_by_path(&source_path).unwrap();
    assert_eq!(note.links.len(), 2);

    // 1. Resolve Heading 1
    let range1 = ws.resolve_link_anchor(&note.links[0]).unwrap();
    assert_eq!(range1.start.line, 0);
    assert_eq!(range1.start.col, 0);

    // 2. Resolve Block 2
    let range2 = ws.resolve_link_anchor(&note.links[1]).unwrap();
    assert_eq!(range2.start.line, 6); // Heading 2 is line 4, block-2 is line 6
    assert_eq!(
        ws.store.get_note(&target_id).unwrap().blocks[1].id,
        "block-2"
    );
}
