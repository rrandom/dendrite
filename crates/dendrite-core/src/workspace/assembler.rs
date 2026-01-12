use crate::syntax::SyntaxStrategy;
use crate::identity::IdentityRegistry;
use crate::model::{Link, Note, NoteId};
use crate::parser::ParseResult;
use std::path::PathBuf;

/// Assembler responsible for converting a raw ParseResult into a semantically enriched Note.
/// It uses a SyntaxStrategy to resolve link targets and an IdentityRegistry to manage IDs.
pub struct NoteAssembler<'a> {
    strategy: &'a dyn SyntaxStrategy,
    identity: &'a mut IdentityRegistry,
}

impl<'a> NoteAssembler<'a> {
    pub fn new(strategy: &'a dyn SyntaxStrategy, identity: &'a mut IdentityRegistry) -> Self {
        Self { strategy, identity }
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
        path: &PathBuf,
        note_id: &NoteId,
    ) -> Note {
        let source_key = self.strategy.note_key_from_path(path, "");

        Note {
            id: note_id.clone(),
            path: Some(path.clone()),
            title: parse_result.title,
            frontmatter: parse_result.frontmatter,
            content_offset: parse_result.content_start_offset,
            links: parse_result
                .links
                .iter()
                .map(|link| {
                    let link_key = self.strategy.note_key_from_link(&source_key, &link.target);
                    Link {
                        target: self.identity.get_or_create(&link_key),
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
