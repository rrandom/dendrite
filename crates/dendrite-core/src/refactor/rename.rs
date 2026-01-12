use crate::line_map::LineMap;
use crate::model::{LinkKind, NoteId};
use crate::refactor::model::{
    Change, ContentProvider, EditGroup, EditPlan, Precondition, RefactorKind, ResourceOperation,
    TextEdit,
};
use crate::store::Store;
use crate::syntax::SyntaxStrategy;

/// Calculate the plan for renaming a note.
///
/// # Arguments
/// * `store` - The workspace store containing knowledge graph
/// * `note_id` - The NoteId of the note being renamed
/// * `new_path` - The target path needed for file system operations
/// * `new_key` - The new note identifier (stem) needed for link text
///
/// # Returns
/// * `Option<EditPlan>` - The plan, or None if the old note doesn't exist.
pub(crate) fn calculate_rename_edits(
    store: &Store,
    content_provider: &dyn ContentProvider,
    strategy: &dyn SyntaxStrategy,
    note_id: &NoteId,
    new_path: std::path::PathBuf,
    new_key: &str,
) -> Option<EditPlan> {
    let note = store.get_note(note_id)?;
    let old_path = note.path.as_ref()?;

    // 2. Preconditions
    let mut preconditions = Vec::new();
    // We expect the caller to have validated ID existence, but we check here for the note object
    preconditions.push(Precondition::NoteExists(note_id.0.to_string()));
    preconditions.push(Precondition::PathNotExists(new_path.clone()));

    let mut edits = Vec::new();

    // 3. Rename the file itself
    let old_uri = old_path.to_string_lossy().to_string();
    let new_uri = new_path.to_string_lossy().to_string();

    edits.push(EditGroup {
        uri: old_uri.clone(),
        changes: vec![Change::ResourceOp(ResourceOperation::RenameFile {
            new_uri: new_uri.clone(),
            overwrite: false,
        })],
    });

    // 4. Update Backlinks
    let backlinks = store.backlinks_of(note_id);
    for source_id in backlinks {
        if let Some(source_note) = store.get_note(&source_id) {
            let source_path = source_note.path.as_ref();
            if source_path.is_none() {
                continue;
            }
            let source_uri = source_path.unwrap().to_string_lossy().to_string();

            let mut changes = Vec::new();

            for link in &source_note.links {
                if link.target == *note_id {
                    let new_text = match link.kind {
                        LinkKind::WikiLink | LinkKind::EmbeddedWikiLink => {
                            // Use strategy to format WikiLink
                            strategy.format_wikilink(
                                new_key,
                                link.alias.as_deref(),
                                link.anchor.as_deref(),
                                link.kind == LinkKind::EmbeddedWikiLink,
                            )
                        }
                        LinkKind::MarkdownLink => {
                            // [alias](path)
                            // We construct a simple relative path assuming flat structure for now,
                            // or rely on caller to canonicalize if needed.
                            // Ideally, markdown links should be path-based, but we use Key.md here.
                            let mut text = String::from("[");
                            if let Some(alias) = &link.alias {
                                text.push_str(alias);
                            } else {
                                text.push_str(new_key);
                            }
                            text.push_str("](");
                            // Best effort: usage of new name with md extension
                            text.push_str(new_key);
                            text.push_str(".md)");
                            text
                        }
                    };

                    let mut undo_text = None;
                    if let Some(content) = content_provider.get_content(&source_uri) {
                        let line_map = LineMap::new(&content);
                        if let Some(start) = line_map.point_to_offset(&content, link.range.start) {
                            if let Some(end) = line_map.point_to_offset(&content, link.range.end) {
                                if start <= end && end <= content.len() {
                                    undo_text = Some(content[start..end].to_string());
                                }
                            }
                        }
                    }

                    changes.push(Change::TextEdit(TextEdit {
                        range: link.range.clone(),
                        new_text,
                        undo_text,
                    }));
                }
            }
            if !changes.is_empty() {
                edits.push(EditGroup {
                    uri: source_uri,
                    changes,
                });
            }
        }
    }

    Some(EditPlan {
        refactor_kind: RefactorKind::RenameNote,
        edits,
        preconditions,
        diagnostics: vec![],
        reversible: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Link, LinkKind, Note, NoteId, TextRange};
    use crate::store::Store;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct MockContentProvider;
    impl ContentProvider for MockContentProvider {
        fn get_content(&self, _uri: &str) -> Option<String> {
            None
        }
    }

    fn create_dummy_note(name: &str) -> crate::model::Note {
        let id = NoteId::new();
        crate::model::Note {
            id,
            path: Some(PathBuf::from(format!("{}.md", name))),
            title: Some(name.to_string()),
            frontmatter: None,
            content_offset: 0,
            links: vec![],
            headings: vec![],
            blocks: vec![],
            digest: None,
        }
    }

    #[test]
    fn test_rename_note_simple() {
        let mut store = Store::new();

        let mut note_a = create_dummy_note("A");
        let note_b = create_dummy_note("B"); // To be renamed

        // A links to B
        note_a.links.push(Link {
            target: note_b.id.clone(),
            alias: None,
            anchor: None,
            range: TextRange::default(),
            kind: LinkKind::WikiLink,
        });

        store.upsert_note(note_a.clone());
        store.upsert_note(note_b.clone());

        // Setup backlinks
        let targets = vec![note_b.id.clone()];
        store.set_outgoing_links(&note_a.id, targets);

        // Perform rename: B -> C
        let new_key = "C";
        let new_path = PathBuf::from("C.md");
        let strategy = crate::syntax::DendronStrategy::new(PathBuf::from("/test"));
        let plan =
            calculate_rename_edits(&store, &MockContentProvider, &strategy, &note_b.id, new_path, new_key)
                .expect("Plan generated");

        assert!(matches!(plan.refactor_kind, RefactorKind::RenameNote));

        // We expect edits for:
        // 1. Rename B.md -> C.md
        // 2. Update A.md link [[B]] -> [[C]]
        assert_eq!(plan.edits.len(), 2);

        // Check RenameFile op
        let rename_group = plan
            .edits
            .iter()
            .find(|g| g.uri.ends_with("B.md"))
            .expect("Rename op not found");

        if let Change::ResourceOp(ResourceOperation::RenameFile { new_uri, .. }) =
            &rename_group.changes[0]
        {
            assert!(new_uri.ends_with("C.md"));
        } else {
            panic!("Expected RenameFile op");
        }

        // Check Link Update
        let link_group = plan
            .edits
            .iter()
            .find(|g| g.uri.ends_with("A.md"))
            .expect("Link update not found");

        if let Change::TextEdit(TextEdit { new_text, .. }) = &link_group.changes[0] {
            assert_eq!(new_text, "[[C]]");
        } else {
            panic!("Expected TextEdit op");
        }
    }

    #[test]
    fn test_rename_design_invariants() {
        // This test verifies the design principles:
        // 1. NoteId is stable (simulated by checking we operate on the same UUID)
        // 2. Note key/filename changes
        // 3. Links are updated to new name

        let mut store = Store::new();
        let note_uuid = Uuid::new_v4();
        let note_id = NoteId(note_uuid);

        // Create note with a specific filename/key "Old Name"
        let note = Note {
            id: note_id.clone(),
            path: Some(PathBuf::from("folder/Old Name.md")),
            title: Some("Old Name".to_string()),
            frontmatter: None,
            content_offset: 0,
            links: vec![],
            headings: vec![],
            blocks: vec![],
            digest: None,
        };
        store.upsert_note(note.clone());

        // Create referencing note
        let ref_note = create_dummy_note("Referencer");
        let mut ref_note = ref_note;
        ref_note.links.push(Link {
            target: note_id.clone(),
            alias: None,
            anchor: None,
            range: TextRange::default(),
            kind: LinkKind::WikiLink,
        });
        ref_note.path = Some(PathBuf::from("folder/Referencer.md")); // Same folder for simplicity
        store.upsert_note(ref_note.clone());
        store.set_outgoing_links(&ref_note.id, vec![note_id.clone()]);

        // Execute Rename: "Old Name" -> "New Name"
        // Caller (Workspace) behavior simulation:
        let new_key = "New Name";
        let new_path = PathBuf::from("folder/New Name.md");

        let strategy = crate::syntax::DendronStrategy::new(PathBuf::from("/test"));
        let plan = calculate_rename_edits(&store, &MockContentProvider, &strategy, &note_id, new_path, new_key)
            .expect("Should generate plan");

        // VERIFICATION 1: Identity Stability
        // We called calculate_rename_edits with the ID directly.
        // The Precondition ensures the note *with this ID* exists.
        assert!(plan
            .preconditions
            .contains(&Precondition::NoteExists(note_uuid.to_string())));

        // VERIFICATION 2: Filename/Key Change
        // The resource op should be RenameFile, changing the path (and thus Key).
        let rename_op = plan
            .edits
            .iter()
            .flat_map(|g| &g.changes)
            .find_map(|c| match c {
                Change::ResourceOp(ResourceOperation::RenameFile { new_uri, .. }) => Some(new_uri),
                _ => None,
            })
            .expect("Must have RenameFile op");

        // Verify path change reflects new name
        assert!(
            rename_op.ends_with("New Name.md"),
            "New URI should end with 'New Name.md', got: {}",
            rename_op
        );
        // Verify it keeps the folder structure (simple check)
        assert!(rename_op.contains("folder"), "Should preserve folder path");

        // VERIFICATION 3: Link Updates
        // The link [[Old Name]] should become [[New Name]]
        let link_edit = plan
            .edits
            .iter()
            .flat_map(|g| &g.changes)
            .find_map(|c| match c {
                Change::TextEdit(edit) => Some(edit),
                _ => None,
            })
            .expect("Must have TextEdit for backlink");

        assert_eq!(
            link_edit.new_text, "[[New Name]]",
            "Link text must update to new key"
        );
    }

    #[test]
    fn test_rename_preserves_blocks() {
        let mut store = Store::new();

        let old_name = "Old Note";
        let new_name = "New Note";
        let new_path = PathBuf::from("New Note.md");

        let old_note = create_dummy_note(old_name);
        let note_id = old_note.id.clone();

        let mut ref_note = create_dummy_note("Referencer");

        // Add link with block anchor: [[Old Note#^block-id]]
        ref_note.links.push(Link {
            target: note_id.clone(),
            alias: None,
            anchor: Some("^block-id".to_string()),
            range: TextRange::default(),
            kind: LinkKind::WikiLink,
        });

        store.upsert_note(old_note);
        store.upsert_note(ref_note.clone());
        store.set_outgoing_links(&ref_note.id, vec![note_id.clone()]);

        let strategy = crate::syntax::DendronStrategy::new(PathBuf::from("/test"));
        let plan = calculate_rename_edits(&store, &MockContentProvider, &strategy, &note_id, new_path, new_name)
            .expect("Plan generation failed");

        // Verify link update
        let link_edit = plan
            .edits
            .iter()
            .flat_map(|g| &g.changes)
            .find_map(|c| match c {
                Change::TextEdit(edit) => Some(edit),
                _ => None,
            })
            .expect("Should have link update");

        assert_eq!(
            link_edit.new_text, "[[New Note#^block-id]]",
            "Block anchor must be preserved"
        );
    }

    #[test]
    fn test_rename_undo_text_extraction() {
        let mut store = Store::new();
        struct MockWithContent;
        impl ContentProvider for MockWithContent {
            fn get_content(&self, uri: &str) -> Option<String> {
                if uri.contains("Referencer.md") {
                    Some("Check [[Old Note]] here.".to_string())
                } else {
                    None
                }
            }
        }

        let old_note = create_dummy_note("Old Note");
        let note_id = old_note.id.clone();
        let mut ref_note = create_dummy_note("Referencer");
        ref_note.path = Some(PathBuf::from("Referencer.md"));

        use crate::model::Point;
        ref_note.links.push(Link {
            target: note_id.clone(),
            alias: None,
            anchor: None,
            range: TextRange {
                start: Point { line: 0, col: 6 },
                end: Point { line: 0, col: 18 },
            },
            kind: LinkKind::WikiLink,
        });

        store.upsert_note(old_note);
        store.upsert_note(ref_note.clone());
        store.set_outgoing_links(&ref_note.id, vec![note_id.clone()]);

        let strategy = crate::syntax::DendronStrategy::new(PathBuf::from("/test"));
        let plan = calculate_rename_edits(
            &store,
            &MockWithContent,
            &strategy,
            &note_id,
            PathBuf::from("New Note.md"),
            "New Note",
        )
        .expect("Should generate plan");

        let link_edit = plan
            .edits
            .iter()
            .flat_map(|g| &g.changes)
            .find_map(|c| match c {
                Change::TextEdit(t) => Some(t),
                _ => None,
            })
            .expect("Should have link update");

        assert_eq!(link_edit.undo_text, Some("[[Old Note]]".to_string()));
        assert_eq!(link_edit.new_text, "[[New Note]]");
        assert!(plan.reversible);
    }
}
