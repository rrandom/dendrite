use super::SemanticModel;
use crate::{
    model::{LinkKind, ModelId, NoteKey, WikiLinkFormat},
    normalize_path_to_id,
};
use std::path::{Path, PathBuf};

pub struct DendronModel {
    root: PathBuf,
}

impl DendronModel {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl SemanticModel for DendronModel {
    fn id(&self) -> ModelId {
        ModelId("Dendron")
    }

    /// Get the root path of the workspace.
    /// This is stored in the specific implementation (e.g. DendronModel)
    /// and is used for resolving absolute paths for diagnostics and FS operations.
    fn root(&self) -> &Path {
        &self.root
    }

    fn note_key_from_path(&self, path: &Path, _: &str) -> NoteKey {
        // Dendron note key is just the filename without .md extension
        // e.g., "foo.bar.md" -> "foo.bar"
        if let Some(file_name) = path.file_stem() {
            file_name.to_string_lossy().to_string()
        } else {
            // Fallback to normalize_path_to_id if no file stem
            normalize_path_to_id(path)
        }
    }

    fn note_key_from_link(&self, source: &NoteKey, raw: &str) -> NoteKey {
        let link_path = Path::new(raw);
        if link_path.is_absolute() || raw.contains('/') || raw.contains('\\') {
            normalize_path_to_id(link_path)
        } else {
            let source_path = Path::new(source);
            if let Some(parent) = source_path.parent() {
                let resolved_path = parent.join(raw);
                normalize_path_to_id(&resolved_path)
            } else {
                normalize_path_to_id(link_path)
            }
        }
    }

    fn resolve_display_name(&self, note: &crate::model::Note) -> String {
        note.title.clone().unwrap_or_default()
    }

    fn resolve_parent(&self, note: &NoteKey) -> Option<NoteKey> {
        // For Dendron: "foo.bar.baz" -> "foo.bar", "foo.bar" -> "foo", "foo" -> None
        let parts: Vec<&str> = note.split('.').collect();
        if parts.len() <= 1 {
            // No parent (root level or single part)
            return None;
        }
        // Return all parts except the last one, joined by '.'
        Some(parts[..parts.len() - 1].join("."))
    }

    fn is_descendant(&self, candidate: &NoteKey, parent: &NoteKey) -> bool {
        // Dendron: candidate starts with "parent."
        // e.g. "a.b" is descendant of "a"
        if candidate.len() <= parent.len() {
            return false;
        }
        candidate.starts_with(parent) && candidate.chars().nth(parent.len()) == Some('.')
    }

    fn reparent_key(&self, key: &NoteKey, old_parent: &NoteKey, new_parent: &NoteKey) -> NoteKey {
        // Replace prefix: "old.child" -> "new.child"
        if !self.is_descendant(key, old_parent) {
            return key.clone();
        }
        let suffix = &key[old_parent.len()..];
        format!("{}{}", new_parent, suffix)
    }

    fn path_from_note_key(&self, key: &NoteKey) -> std::path::PathBuf {
        // Generate full path: root / "key.md"
        // e.g., root = "/workspace", key = "foo.bar" -> "/workspace/foo.bar.md"
        self.root.join(format!("{}.md", key))
    }

    fn format_wikilink(
        &self,
        target: &str,
        alias: Option<&str>,
        anchor: Option<&str>,
        is_embed: bool,
    ) -> String {
        let mut out = String::from(if is_embed { "![[" } else { "[[" });

        // Dendron: [[alias|target#anchor]]
        if let Some(a) = alias {
            out.push_str(a);
            out.push('|');
        }

        out.push_str(target);

        if let Some(anc) = anchor {
            if !anc.starts_with('#') {
                out.push('#');
            }
            out.push_str(anc);
        }

        out.push_str("]]");
        out
    }

    fn supported_link_kinds(&self) -> Vec<LinkKind> {
        vec![
            LinkKind::WikiLink(WikiLinkFormat::AliasFirst),
            LinkKind::EmbeddedWikiLink(WikiLinkFormat::AliasFirst),
            LinkKind::MarkdownLink,
        ]
    }

    fn supported_extensions(&self) -> &[&str] {
        &["md"]
    }
}
