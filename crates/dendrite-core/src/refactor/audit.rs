use crate::refactor::model::{EditPlan, RefactorKind};
use crate::semantic::SemanticModel;
use crate::store::Store;

/// Audit the entire workspace for reference graph health.
///
/// Scans for:
/// 1. Broken links (missing .md files)
/// 2. Invalid anchors (missing headings/blocks)
/// 3. Model-strict syntax violations (e.g. [[#abc]] in Dendron)
pub fn calculate_audit_diagnostics(
    store: &Store,
    model: &dyn SemanticModel,
) -> crate::refactor::model::EditPlan {
    use crate::refactor::model::{Diagnostic, DiagnosticSeverity};
    let mut diagnostics = Vec::new();

    let audited_kinds = model.audited_link_kinds();

    for note in store.all_notes() {
        // ... (existing read content code)
        let uri = note.path.as_ref().map(|p| p.to_string_lossy().to_string());

        for link in &note.links {
            // Check if model wants to audit this link kind
            if !audited_kinds.contains(&link.kind) {
                continue;
            }
            let lower_target = link.raw_target.to_lowercase();
            if lower_target.starts_with("http://")
                || lower_target.starts_with("https://")
                || lower_target.starts_with("mailto:")
            {
                continue;
            }

            let mut is_broken = false;

            let target_note = store.get_note(&link.target);

            if target_note.is_none() {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "Broken link: Target note not found.".to_string(),
                    uri: uri.clone(),
                    range: Some(link.range.clone()),
                });
                is_broken = true;
            }

            // 2. Anchor Validity Check (only if link is not broken)
            if !is_broken {
                if let (Some(target), Some(anchor)) = (target_note, &link.anchor) {
                    let mut found = false;

                    // Check headings
                    if anchor.starts_with('^') {
                        // Block ID
                        if target.blocks.iter().any(|b| b.id == *anchor) {
                            found = true;
                        }
                    } else {
                        // Heading or Section
                        if target.headings.iter().any(|h| h.text == *anchor) {
                            found = true;
                        }
                    }

                    if !found {
                        diagnostics.push(Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            message: format!(
                                "Invalid anchor: '{}' not found in target note.",
                                anchor
                            ),
                            uri: uri.clone(),
                            range: Some(link.range.clone()),
                        });
                    }
                }
            }

            // 3. Model-strict syntax validation (e.g. Dendron bare anchors)
            if model.id().0 == "Dendron" && link.raw_target.starts_with('#') {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!("Dendron strictly forbids bare anchor links like '{}'. Use '[[note#anchor]]'.", link.raw_target),
                    uri: uri.clone(),
                    range: Some(link.range.clone()),
                });
            }
        }
    }

    EditPlan {
        refactor_kind: RefactorKind::WorkspaceAudit,
        edits: vec![],
        preconditions: vec![],
        diagnostics,
        reversible: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityRegistry;
    use crate::model::{Heading, Link, LinkKind, Note, TextRange};
    use crate::semantic::DendronModel;
    use std::path::PathBuf;

    #[test]
    fn test_audit_broken_link() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        let id_a = identity.get_or_create(&"A".to_string());
        let id_missing = identity.get_or_create(&"Missing".to_string());

        let mut note_a = Note {
            id: id_a.clone(),
            path: Some(PathBuf::from("A.md")),
            title: Some("A".to_string()),
            ..Default::default()
        };

        note_a.links.push(Link {
            target: id_missing.clone(),
            raw_target: "Missing".to_string(),
            alias: None,
            anchor: None,
            range: TextRange::default(),
            kind: LinkKind::WikiLink(crate::model::WikiLinkFormat::AliasFirst),
        });

        store.upsert_note(note_a);

        let model = DendronModel::new(PathBuf::from("/test"));
        let plan = calculate_audit_diagnostics(&store, &model);

        assert_eq!(plan.diagnostics.len(), 1);
        assert!(plan.diagnostics[0].message.contains("Broken link"));
    }

    #[test]
    fn test_audit_invalid_anchor() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        let id_a = identity.get_or_create(&"A".to_string());
        let id_target = identity.get_or_create(&"Target".to_string());

        let note_target = Note {
            id: id_target.clone(),
            path: Some(PathBuf::from("Target.md")),
            headings: vec![Heading {
                text: "Existing".to_string(),
                level: 1,
                range: TextRange::default(),
            }],
            ..Default::default()
        };

        let mut note_a = Note {
            id: id_a.clone(),
            path: Some(PathBuf::from("A.md")),
            ..Default::default()
        };

        note_a.links.push(Link {
            target: id_target.clone(),
            raw_target: "Target".to_string(),
            alias: None,
            anchor: Some("NonExistent".to_string()),
            range: TextRange::default(),
            kind: LinkKind::WikiLink(crate::model::WikiLinkFormat::AliasFirst),
        });

        store.upsert_note(note_target);
        store.upsert_note(note_a);

        let model = DendronModel::new(PathBuf::from("/test"));
        let plan = calculate_audit_diagnostics(&store, &model);

        assert_eq!(plan.diagnostics.len(), 1);
        assert!(plan.diagnostics[0].message.contains("Invalid anchor"));
    }

    #[test]
    fn test_audit_dendron_bare_anchor() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        let id_a = identity.get_or_create(&"A".to_string());

        let mut note_a = Note {
            id: id_a.clone(),
            path: Some(PathBuf::from("A.md")),
            ..Default::default()
        };

        note_a.links.push(Link {
            target: id_a.clone(),
            raw_target: "#forbidden".to_string(),
            alias: None,
            anchor: Some("forbidden".to_string()),
            range: TextRange::default(),
            kind: LinkKind::WikiLink(crate::model::WikiLinkFormat::AliasFirst),
        });

        store.upsert_note(note_a);

        let model = DendronModel::new(PathBuf::from("/test"));
        let plan = calculate_audit_diagnostics(&store, &model);

        // It might have 2 diagnostics: "Invalid anchor" AND "Bare anchor error"
        assert!(plan.diagnostics.len() >= 1);
        assert!(plan
            .diagnostics
            .iter()
            .any(|d| d.message.contains("strictly forbids bare anchor")));
    }
}
