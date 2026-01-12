use crate::identity::IdentityRegistry;
use crate::model::{NoteId, TextRange};
use crate::refactor::model::{
    Change, ContentProvider, EditGroup, EditPlan, RefactorKind, ResourceOperation, TextEdit,
};
use crate::semantic::SemanticModel;
use crate::store::Store;

/// Calculate edits for "Extract Selection to Note" (SplitNote).
///
/// 1. Extracts text from `source_note` at `selection`.
/// 2. Creates a new note file at `new_path` with the extracted text.
/// 3. Replaces `selection` in `source_note` with a link to `new_note_title`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn calculate_split_edits(
    store: &Store,
    _identity: &IdentityRegistry,
    content_provider: &dyn ContentProvider,
    strategy: &dyn SemanticModel,
    source_id: &NoteId,
    selection: TextRange,
    new_note_title: &str,
) -> Option<EditPlan> {
    // 1. Validation and Setup
    let source_note = store.get_note(source_id)?;
    let source_path = source_note.path.as_ref()?;
    let source_uri = source_path.to_string_lossy().to_string();

    // 2. Read Source Content
    let source_content = content_provider.get_content(&source_uri)?;

    // 3. Extract Text
    let extracted_text = extract_text(&source_content, selection)?;

    // 4. Calculate New Path from Title (Model-Driven)
    let new_path = strategy.path_from_note_key(&new_note_title.to_string());

    // 5. Generate Link Text
    let link_text = strategy.format_wikilink(new_note_title, None, None, false);

    // 6. Prepare Edits
    let mut edits = Vec::new();

    // Op 1: Create New File
    let new_uri = new_path.to_string_lossy().to_string();
    edits.push(EditGroup {
        uri: new_uri.clone(),
        changes: vec![Change::ResourceOp(ResourceOperation::CreateFile {
            content: Some(extracted_text.clone()),
        })],
    });

    // Op 2: Update Source File (Replace selection with link)
    edits.push(EditGroup {
        uri: source_uri,
        changes: vec![Change::TextEdit(TextEdit {
            range: selection,
            new_text: link_text,
            undo_text: Some(extracted_text),
        })],
    });

    Some(EditPlan {
        refactor_kind: RefactorKind::SplitNote,
        edits,
        preconditions: vec![],
        diagnostics: vec![],
        reversible: true,
    })
}

/// Helper to extract text from content using TextRange (0-based line/col).
fn extract_text(content: &str, range: TextRange) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    if start_line >= lines.len() || end_line >= lines.len() {
        return None;
    }

    if start_line == end_line {
        let line = lines[start_line];
        let start_col = range.start.col as usize;
        let end_col = range.end.col as usize;
        if start_col > line.len() || end_col > line.len() {
            return None;
        }
        // Be careful with unicode boundaries here in real impl, mostly ascii for MVP testing
        return Some(line[start_col..end_col].to_string());
    }

    let mut result = String::new();

    // First line
    let first_line = lines[start_line];
    if range.start.col as usize <= first_line.len() {
        result.push_str(&first_line[range.start.col as usize..]);
        result.push('\n');
    }

    // Middle lines
    for i in (start_line + 1)..end_line {
        result.push_str(lines[i]);
        result.push('\n');
    }

    // Last line
    let last_line = lines[end_line];
    if range.end.col as usize <= last_line.len() {
        result.push_str(&last_line[..range.end.col as usize]);
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Note, Point};
    use std::path::PathBuf;

    struct MockProvider {
        content: String,
    }

    impl ContentProvider for MockProvider {
        fn get_content(&self, _uri: &str) -> Option<String> {
            Some(self.content.clone())
        }
    }

    #[test]
    fn test_extract_selection() {
        let mut store = Store::new();
        let mut identity = IdentityRegistry::new();

        // Setup Source Note
        let id_a = identity.get_or_create(&"A".to_string());
        let note_a = Note {
            id: id_a.clone(),
            path: Some(PathBuf::from("source.md")),
            ..Default::default()
        };
        store.upsert_note(note_a);

        let content = "Line 1\nTarget Text\nLine 3".to_string();
        let provider = MockProvider { content };

        let strategy = crate::semantic::DendronModel::new(PathBuf::from("/"));

        // Selection: "Target Text" (Line 1, Col 0 to Line 1, Col 11)
        let selection = TextRange {
            start: Point { line: 1, col: 0 },
            end: Point { line: 1, col: 11 },
        };

        let plan = calculate_split_edits(
            &store, &identity, &provider, &strategy, &id_a, selection, "target",
        )
        .expect("Plan generated");

        // Verify Source Update
        let source_edit = plan
            .edits
            .iter()
            .find(|g| g.uri.ends_with("source.md"))
            .unwrap();
        if let Change::TextEdit(edit) = &source_edit.changes[0] {
            assert_eq!(edit.new_text, "[[target]]");
            assert_eq!(edit.range, selection);
        } else {
            panic!("Expected text edit on source");
        }

        // Verify New File Creation
        let new_file = plan
            .edits
            .iter()
            .find(|g| g.uri.ends_with("target.md"))
            .unwrap();
        if let Change::ResourceOp(ResourceOperation::CreateFile { content, .. }) =
            &new_file.changes[0]
        {
            assert_eq!(content.as_deref(), Some("Target Text"));
        } else {
            panic!("Expected create file op");
        }
    }
}
