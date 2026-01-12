use crate::identity::IdentityRegistry;
use crate::line_map::LineMap;
use crate::model::{LinkKind, NoteId};
use crate::refactor::model::{
    Change, ContentProvider, EditGroup, EditPlan, Precondition, RefactorKind, ResourceOperation,
    TextEdit,
};
use crate::store::Store;
use crate::semantic::SemanticModel;
use std::path::{Path, PathBuf};

/// Calculate the plan for structural changes to a note (Rename and/or Move).
///
/// # Arguments
/// * `store` - The workspace store containing knowledge graph
/// * `note_id` - The NoteId of the note being moved/renamed
/// * `new_path` - The target physical path
/// * `new_key` - The target semantic key
///
/// # Returns
/// * `Option<EditPlan>` - The plan, or None if the old note doesn't exist.
pub(crate) fn calculate_structural_edits(
    store: &Store,
    identity: &IdentityRegistry,
    content_provider: &dyn ContentProvider,
    strategy: &dyn SemanticModel,
    note_id: &NoteId,
    new_path: std::path::PathBuf,
    new_key: &str,
) -> Option<EditPlan> {
    let note = store.get_note(note_id)?;
    let old_path = note.path.as_ref()?;
    let (_, old_key) = identity.key_of(note_id)?;

    let is_rename = old_key != new_key;
    let is_move = old_path != &new_path;

    if !is_rename && !is_move {
        return None;
    }

    // 2. Preconditions
    let mut preconditions = Vec::new();
    preconditions.push(Precondition::NoteExists(note_id.0.to_string()));
    if is_move {
        preconditions.push(Precondition::PathNotExists(new_path.clone()));
    }

    let mut edits = Vec::new();

    // 3. Handle File System Change
    let old_uri = old_path.to_string_lossy().to_string();
    let new_uri = new_path.to_string_lossy().to_string();

    if is_move {
        edits.push(EditGroup {
            uri: old_uri.clone(),
            changes: vec![Change::ResourceOp(ResourceOperation::RenameFile {
                new_uri: new_uri.clone(),
                overwrite: false,
            })],
        });
    }

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
                    let mut needs_update = false;
                    let mut new_text = String::new();

                    match link.kind {
                        LinkKind::WikiLink | LinkKind::EmbeddedWikiLink => {
                            if is_rename {
                                needs_update = true;
                                new_text = strategy.format_wikilink(
                                    new_key,
                                    link.alias.as_deref(),
                                    link.anchor.as_deref(),
                                    link.kind == LinkKind::EmbeddedWikiLink,
                                );
                            }
                        }
                        LinkKind::MarkdownLink => {
                            if is_rename || is_move {
                                needs_update = true;
                                let mut text = String::from("[");
                                if let Some(alias) = &link.alias {
                                    text.push_str(alias);
                                } else {
                                    text.push_str(new_key);
                                }
                                text.push_str("](");
                                
                                // Calculate relative path if we have both paths
                                if let Some(source_path) = source_note.path.as_ref() {
                                    let rel_path = calculate_relative_path(source_path, &new_path);
                                    let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                                    text.push_str(&rel_str);
                                } else {
                                    // Fallback to simple key-based path
                                    let ext = strategy.supported_extensions().first().unwrap_or(&"md");
                                    text.push_str(new_key);
                                    text.push('.');
                                    text.push_str(ext);
                                }
                                
                                text.push(')');
                                new_text = text;
                            }
                        }
                    }

                    if needs_update {
                        let mut undo_text = None;
                        if let Some(content) = content_provider.get_content(&source_uri) {
                            let line_map = LineMap::new(&content);
                            if let Some(start) = line_map.point_to_offset(&content, link.range.start)
                            {
                                if let Some(end) = line_map.point_to_offset(&content, link.range.end)
                                {
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
        refactor_kind: if is_rename {
            RefactorKind::RenameNote
        } else {
            RefactorKind::MoveNote
        },
        edits,
        preconditions,
        diagnostics: vec![],
        reversible: true,
    })
}

fn calculate_relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_dir = from.parent().unwrap_or(Path::new(""));
    
    let from_comps: Vec<_> = from_dir.components().collect();
    let to_comps: Vec<_> = to.components().collect();
    
    let mut common_count = 0;
    for (f, t) in from_comps.iter().zip(to_comps.iter()) {
        if f == t {
            common_count += 1;
        } else {
            break;
        }
    }
    
    let mut result = PathBuf::new();
    for _ in 0..(from_comps.len() - common_count) {
        result.push("..");
    }
    for comp in &to_comps[common_count..] {
        result.push(comp);
    }
    
    if result.as_os_str().is_empty() {
        if let Some(filename) = to.file_name() {
            PathBuf::from(filename)
        } else {
            PathBuf::from(".")
        }
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Link, LinkKind, NoteId, TextRange};
    use crate::store::Store;
    use std::path::PathBuf;

    struct MockContentProvider;
    impl ContentProvider for MockContentProvider {
        fn get_content(&self, _uri: &str) -> Option<String> {
            None
        }
    }

    fn create_dummy_note(id: NoteId, name: &str) -> crate::model::Note {
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
        let mut identity = IdentityRegistry::new();

        let id_a = identity.get_or_create(&"A".to_string());
        let id_b = identity.get_or_create(&"B".to_string());

        let note_a = create_dummy_note(id_a.clone(), "A");
        let note_b = create_dummy_note(id_b.clone(), "B");

        let mut note_a = note_a;
        note_a.links.push(Link {
            target: id_b.clone(),
            alias: None,
            anchor: None,
            range: TextRange::default(),
            kind: LinkKind::WikiLink,
        });

        store.upsert_note(note_a.clone());
        store.upsert_note(note_b.clone());
        store.set_outgoing_links(&id_a, vec![id_b.clone()]);

        let new_key = "C";
        let new_path = PathBuf::from("C.md");
        let strategy = crate::semantic::DendronModel::new(PathBuf::from("/test"));
        let plan = calculate_structural_edits(
            &store,
            &identity,
            &MockContentProvider,
            &strategy,
            &id_b,
            new_path,
            new_key,
        )
        .expect("Plan generated");

        assert!(matches!(plan.refactor_kind, RefactorKind::RenameNote));
        assert_eq!(plan.edits.len(), 2);

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
    fn test_move_note_without_rename() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        let id_a = identity.get_or_create(&"A".to_string());
        let note_a = create_dummy_note(id_a.clone(), "A");

        store.upsert_note(note_a.clone());

        // Move A.md to sub/A.md. Key stays "A" in some models (but Dendron would change it).
        // Let's assume we are moving and the Key STAYS THE SAME.
        let new_key = "A"; 
        let new_path = PathBuf::from("sub/A.md");
        let strategy = crate::semantic::DendronModel::new(PathBuf::from("/test"));
        
        // Note: DendronModel usually derives Key from path, so if we pass "A" but new_path is "sub/A.md",
        // calculate_structural_edits will see "A" (old) vs "A" (new) -> No Rename.
        
        let plan = calculate_structural_edits(
            &store,
            &identity,
            &MockContentProvider,
            &strategy,
            &id_a,
            new_path,
            new_key,
        )
        .expect("Plan generated");

        assert!(matches!(plan.refactor_kind, RefactorKind::MoveNote));
        // We only expect 1 edit: the file rename
        assert_eq!(plan.edits.len(), 1);
        
        let move_group = &plan.edits[0];
        assert!(move_group.uri.ends_with("A.md"));
        if let Change::ResourceOp(ResourceOperation::RenameFile { new_uri, .. }) = &move_group.changes[0] {
            assert!(new_uri.contains("sub"));
        } else {
            panic!("Expected RenameFile op");
        }
    }

    #[test]
    fn test_rename_preserves_blocks() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        let id_old = identity.get_or_create(&"Old Note".to_string());
        let id_ref = identity.get_or_create(&"Referencer".to_string());

        let note_old = create_dummy_note(id_old.clone(), "Old Note");
        let mut note_ref = create_dummy_note(id_ref.clone(), "Referencer");

        note_ref.links.push(Link {
            target: id_old.clone(),
            alias: None,
            anchor: Some("^block-id".to_string()),
            range: TextRange::default(),
            kind: LinkKind::WikiLink,
        });

        store.upsert_note(note_old);
        store.upsert_note(note_ref.clone());
        store.set_outgoing_links(&id_ref, vec![id_old.clone()]);

        let strategy = crate::semantic::DendronModel::new(PathBuf::from("/test"));
        let plan = calculate_structural_edits(
            &store,
            &identity,
            &MockContentProvider,
            &strategy,
            &id_old,
            PathBuf::from("New Note.md"),
            "New Note",
        )
        .expect("Plan generation failed");

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
    fn test_move_cross_folder_markdown_link() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        let id_target = identity.get_or_create(&"Target".to_string());
        let id_source = identity.get_or_create(&"Source".to_string());

        let note_target = create_dummy_note(id_target.clone(), "Target");
        let mut note_source = create_dummy_note(id_source.clone(), "Source");
        
        // Source is in root/source dir, Target is moved from root/Target.md to root/sub/Target.md
        note_source.path = Some(PathBuf::from("docs/Source.md"));
        
        note_source.links.push(Link {
            target: id_target.clone(),
            alias: None,
            anchor: None,
            range: TextRange::default(),
            kind: LinkKind::MarkdownLink,
        });

        store.upsert_note(note_target);
        store.upsert_note(note_source.clone());
        store.set_outgoing_links(&id_source, vec![id_target.clone()]);

        let new_path = PathBuf::from("archive/Target.md");
        let new_key = "Target"; // Key stays same
        
        let strategy = crate::semantic::DendronModel::new(PathBuf::from("/test"));
        let plan = calculate_structural_edits(
            &store,
            &identity,
            &MockContentProvider,
            &strategy,
            &id_target,
            new_path,
            new_key,
        )
        .expect("Plan generated");

        let link_edit = plan
            .edits
            .iter()
            .flat_map(|g| &g.changes)
            .find_map(|c| match c {
                Change::TextEdit(edit) => Some(edit),
                _ => None,
            })
            .expect("Should have link update");

        // From docs/Source.md to archive/Target.md -> ../archive/Target.md
        assert_eq!(
            link_edit.new_text, "[Target](../archive/Target.md)",
            "Markdown link should use relative path"
        );
    }
}

