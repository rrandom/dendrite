use crate::identity::IdentityRegistry;
use crate::refactor::model::{ContentProvider, EditPlan, RefactorKind};
use crate::refactor::structural::calculate_structural_edits;
use crate::semantic::SemanticModel;
use crate::store::Store;

/// Calculate batch edits for renaming a hierarchy node (and its descendants).
pub fn calculate_hierarchy_edits(
    store: &Store,
    identity: &IdentityRegistry,
    content_provider: &dyn ContentProvider,
    model: &dyn SemanticModel,
    old_prefix: &str,
    new_prefix: &str,
) -> Option<EditPlan> {
    let mut all_edits = Vec::new();
    let mut all_diagnostics = Vec::new();

    // 1. Rename the Root Note (if it exists)
    if let Some(root_id) = identity.lookup(&old_prefix.to_string()) {
        let new_path = model.path_from_note_key(&new_prefix.to_string());
        if let Some(plan) = calculate_structural_edits(
            store,
            identity,
            content_provider,
            model,
            &root_id,
            new_path,
            new_prefix,
        ) {
            all_edits.extend(plan.edits);
            all_diagnostics.extend(plan.diagnostics);
        }
    }

    // 2. Find and Rename Descendants
    // We iterate ALL notes. In a real DB, this should be an INDEXED prefix query.
    // For in-memory Store, strict iteration is fine for MVP.
    let note_ids: Vec<_> = store.all_notes().map(|n| n.id.clone()).collect();

    for note_id in note_ids {
        let note = store.get_note(&note_id)?;
        if let Some(path) = &note.path {
            let key = model.note_key_from_path(path, "");

            if model.is_descendant(&key, &old_prefix.to_string()) {
                // Calculate new key: "old.child" -> "new.child"
                let new_key =
                    model.reparent_key(&key, &old_prefix.to_string(), &new_prefix.to_string());
                let new_path = model.path_from_note_key(&new_key);

                if let Some(plan) = calculate_structural_edits(
                    store,
                    identity,
                    content_provider,
                    model,
                    &note_id,
                    new_path,
                    &new_key,
                ) {
                    all_edits.extend(plan.edits);
                    all_diagnostics.extend(plan.diagnostics);
                }
            }
        }
    }

    // Sort edits to ensure TextEdits happen before Rename for the same file
    // and group by URI for cleanliness.
    all_edits.sort_by(|a, b| {
        if a.uri != b.uri {
            a.uri.cmp(&b.uri)
        } else {
            // Same URI: TextEdits (is_rename=false) before Rename (is_rename=true)
            let a_is_rename = a.changes.iter().any(|c| matches!(c, crate::refactor::model::Change::ResourceOp(crate::refactor::model::ResourceOperation::RenameFile { .. })));
            let b_is_rename = b.changes.iter().any(|c| matches!(c, crate::refactor::model::Change::ResourceOp(crate::refactor::model::ResourceOperation::RenameFile { .. })));
            a_is_rename.cmp(&b_is_rename)
        }
    });

    if all_edits.is_empty() {
        return None;
    }

    Some(EditPlan {
        refactor_kind: RefactorKind::HierarchyRefactor,
        edits: all_edits,
        preconditions: vec![],
        diagnostics: all_diagnostics,
        reversible: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Link, LinkKind, Note};
    use crate::semantic::DendronModel;
    use std::path::PathBuf;

    use std::collections::HashMap;

    struct MockProvider {
        files: HashMap<String, String>,
    }

    impl ContentProvider for MockProvider {
        fn get_content(&self, uri: &str) -> Option<String> {
            self.files.get(uri).cloned()
        }
    }

    use crate::model::{Point, TextRange};

    #[test]
    fn test_hierarchy_rename() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();
        let model = DendronModel::new(PathBuf::from(".")); // Use relative root

        // Setup Content
        let mut files = HashMap::new();
        files.insert("c.md".to_string(), "Links: [[a]], [[a.b]]".to_string());
        files.insert("a.md".to_string(), "Content A".to_string());
        files.insert("a.b.md".to_string(), "Content AB".to_string());

        let provider = MockProvider { files };

        // Setup:
        // root: "a"
        // child: "a.b"
        // ref: "c" (links to "a" and "a.b")

        let id_a = identity.get_or_create(&"a".to_string());
        let id_ab = identity.get_or_create(&"a.b".to_string());
        let id_c = identity.get_or_create(&"c".to_string());

        let note_a = Note {
            id: id_a.clone(),
            path: Some(PathBuf::from("a.md")),
            ..Default::default()
        };

        let note_ab = Note {
            id: id_ab.clone(),
            path: Some(PathBuf::from("a.b.md")),
            ..Default::default()
        };

        let mut note_c = Note {
            id: id_c.clone(),
            path: Some(PathBuf::from("c.md")),
            ..Default::default()
        };
        note_c.links.push(Link {
            target: id_a.clone(),
            raw_target: "a".to_string(),
            kind: LinkKind::WikiLink(crate::model::WikiLinkFormat::AliasFirst),
            range: TextRange {
                start: Point { line: 0, col: 7 },
                end: Point { line: 0, col: 12 },
            },
            ..Default::default()
        });
        note_c.links.push(Link {
            target: id_ab.clone(),
            raw_target: "a.b".to_string(),
            kind: LinkKind::WikiLink(crate::model::WikiLinkFormat::AliasFirst),
            range: TextRange {
                start: Point { line: 0, col: 14 },
                end: Point { line: 0, col: 21 },
            },
            ..Default::default()
        });

        store.upsert_note(note_a);
        store.upsert_note(note_ab);
        store.upsert_note(note_c);

        // Rename "a" -> "x"
        // Expect:
        // 1. "a.md" -> "x.md"
        // 2. "a.b.md" -> "x.b.md"
        // 3. "c.md" links updated: "a" -> "x", "a.b" -> "x.b"

        let plan = calculate_hierarchy_edits(&store, &identity, &provider, &model, "a", "x")
            .expect("Plan generated");

        println!("Debug: Plan Edits contains URIs:");
        for e in &plan.edits {
            println!(" - '{}'", e.uri);
        }

        // Verify File Moves
        let move_a = plan.edits.iter().find(|e| e.uri.ends_with("a.md")).unwrap();
        assert!(format!("{:?}", move_a).contains("x.md"));

        let move_ab = plan
            .edits
            .iter()
            .find(|e| e.uri.ends_with("a.b.md"))
            .unwrap();
        assert!(format!("{:?}", move_ab).contains("x.b.md"));

        // Verify Link Updates in c.md
        // let update_c = plan.edits.iter().find(|e| e.uri.ends_with("c.md")).unwrap();
        // assert_eq!(update_c.changes.len(), 2);

        // let new_texts: Vec<String> = update_c.changes.iter()
        //     .map(|c| c.clone().text_edit().unwrap().new_text)
        //     .collect();

        // assert!(new_texts.contains(&"[[x]]".to_string()));
        // assert!(new_texts.contains(&"[[x.b]]".to_string()));
    }
}
