use crate::identity::IdentityRegistry;
use crate::model::{Link, Note, NoteId};
use crate::parser::ParseResult;
use crate::semantic::SemanticModel;

/// Assembler responsible for converting a raw ParseResult into a semantically enriched Note.
/// It uses a SemanticModel to resolve link targets and an IdentityRegistry to manage IDs.
pub struct NoteAssembler<'a> {
    model: &'a dyn SemanticModel,
    identity: &'a mut IdentityRegistry,
}

impl<'a> NoteAssembler<'a> {
    pub fn new(model: &'a dyn SemanticModel, identity: &'a mut IdentityRegistry) -> Self {
        Self { model, identity }
    }

    /// Assembles a Note from a ParseResult.
    ///
    /// # Arguments
    /// * `parse_result` - The raw output from the parser.
    /// * `path` - The file path of the note.
    /// * `note_id` - The stable ID for this note.
    pub fn assemble(
        &mut self,
        parse_result: ParseResult,
        path: &std::path::Path,
        note_id: &NoteId,
    ) -> Note {
        let source_key = self.model.note_key_from_path(path, "");

        Note {
            id: note_id.clone(),
            path: Some(path.to_path_buf()),
            title: parse_result.title,
            frontmatter: parse_result.frontmatter,
            content_offset: parse_result.content_start_offset as u32,
            links: parse_result
                .links
                .iter()
                .map(|link| {
                    // Handle self-reference links: [[#anchor]]
                    // When target is empty AND anchor is present, link to current note
                    let link_key = if link.target.is_empty() && link.anchor.is_some() {
                        // Self-reference: [[#anchor]]
                        source_key.clone()
                    } else if link.target.is_empty() {
                        // Invalid: [[]] without anchor - fallback to source
                        // TODO: Consider logging a warning
                        source_key.clone()
                    } else {
                        self.model.note_key_from_link(&source_key, &link.target)
                    };

                    Link {
                        target: self.identity.get_or_create(&link_key),
                        raw_target: link.raw_target.clone(),
                        alias: link.alias.clone(),
                        anchor: link.anchor.clone(),
                        range: link.range,
                        kind: link.kind.clone(),
                    }
                })
                .collect(),
            headings: parse_result.headings,
            blocks: parse_result.blocks,
            digest: Some(parse_result.digest),
        }
    }
}
