//! Conversion utilities between Core types and LSP types
//!
//! This module will contain functions to convert between Core's internal types
//! (Point, TextRange) and LSP types (Position, Range) when needed.

use dendrite_core::model::{Point, TextRange};
use dendrite_core::refactor::model::{Change, EditPlan, ResourceOperation};
use dendrite_core::refactor::model::{
    Diagnostic as CoreDiagnostic, DiagnosticSeverity as CoreSeverity,
};
use std::path::PathBuf;
use tower_lsp::lsp_types::{
    CreateFile, CreateFileOptions, DeleteFile, DeleteFileOptions, DocumentChangeOperation, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, RenameFile, RenameFileOptions,
    ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity};

/// Convert Core Diagnostic to LSP Diagnostic
pub fn core_diagnostic_to_lsp_diagnostic(
    diag: CoreDiagnostic,
    root_path: Option<&std::path::Path>,
) -> Option<(Url, Diagnostic)> {
    let uri_str = diag.uri?;
    let url = match Url::parse(&uri_str) {
        Ok(u) if u.scheme() == "file" => u,
        _ => {
            // If it fails to parse as URL, try as file path
            let path = PathBuf::from(&uri_str);
            if path.is_absolute() {
                Url::from_file_path(path).ok()?
            } else if let Some(root) = root_path {
                // Ensure we append to root, even if path starts with separator
                let relative_str = uri_str.trim_start_matches(['/', '\\']);
                let absolute = root.join(relative_str);

                // Try to canonicalize to resolve symlinks and ensure proper drive letter casing
                let final_path = if let Ok(canon) = std::fs::canonicalize(&absolute) {
                    canon
                } else {
                    absolute
                };

                Url::from_file_path(final_path).ok()?
            } else {
                return None;
            }
        }
    };

    let range = diag.range.map(text_range_to_lsp_range).unwrap_or_default();

    let severity = match diag.severity {
        CoreSeverity::Error => DiagnosticSeverity::ERROR,
        CoreSeverity::Warning => DiagnosticSeverity::WARNING,
        CoreSeverity::Info => DiagnosticSeverity::INFORMATION,
    };

    Some((
        url,
        Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("dendrite".to_string()),
            message: diag.message,
            related_information: None,
            tags: None,
            data: None,
        },
    ))
}

/// Convert LSP Position to Core Point
/// LSP uses 0-based line and character (UTF-16 code units)
/// Core uses 0-based line and column (UTF-8 byte offsets)
pub fn lsp_position_to_point(position: Position) -> Point {
    Point {
        line: position.line,
        col: position.character,
    }
}

/// Convert Core Point to LSP Position
pub fn point_to_lsp_position(point: Point) -> Position {
    Position {
        line: point.line,
        character: point.col,
    }
}

/// Convert Core TextRange to LSP Range
pub fn text_range_to_lsp_range(range: TextRange) -> Range {
    Range {
        start: point_to_lsp_position(range.start),
        end: point_to_lsp_position(range.end),
    }
}

/// Convert PathBuf to LSP Url
pub fn path_to_uri(path: &PathBuf) -> Option<Url> {
    Url::from_file_path(path).ok()
}

/// Convert EditPlan to LSP WorkspaceEdit
pub fn edit_plan_to_workspace_edit(plan: EditPlan) -> WorkspaceEdit {
    let mut document_changes = Vec::new();

    for group in plan.edits {
        let uri = match Url::parse(&group.uri) {
            Ok(u) if u.scheme() == "file" => u,
            _ => {
                // Try as file path
                let path = PathBuf::from(&group.uri);
                match Url::from_file_path(path) {
                    Ok(u) => u,
                    Err(_) => continue,
                }
            }
        };

        // Separate ResourceOps from TextEdits
        // LSP structure: documentChanges is a list of operations.
        // A TextDocumentEdit groups multiple TextEdits for ONE document.
        // ResourceOps are standalone operations.

        // We need to group contiguous TextEdits into one TextDocumentEdit if possible,
        // but our EditGroup is already per-file.

        // Re-implementing correctly:
        // Dendrite's EditGroup contains a mix of changes for a Single URI.
        // But ResourceOp (Rename/Move) might change the URI?
        // Actually `EditGroup` has `uri`. If it's a RenameFile, it's usually "Rename from `uri` to `new_uri`".

        let mut current_text_edits = Vec::new();

        for change in group.changes {
            match change {
                Change::TextEdit(edit) => {
                    current_text_edits.push(TextEdit {
                        range: text_range_to_lsp_range(edit.range),
                        new_text: edit.new_text,
                    });
                }
                Change::ResourceOp(op) => {
                    // 1. Flush pending text edits
                    if !current_text_edits.is_empty() {
                        document_changes.push(DocumentChangeOperation::Edit(TextDocumentEdit {
                            text_document: OptionalVersionedTextDocumentIdentifier {
                                uri: uri.clone(),
                                version: None,
                            },
                            edits: current_text_edits
                                .into_iter()
                                .map(OneOf::Left)
                                .collect(),
                        }));
                        current_text_edits = Vec::new();
                    }

                    // 2. Emit Resource Op
                    match op {
                        ResourceOperation::RenameFile { new_uri, overwrite } => {
                            let new_url = Url::parse(&new_uri)
                                .ok()
                                .filter(|u| u.scheme() == "file")
                                .or_else(|| Url::from_file_path(PathBuf::from(&new_uri)).ok());

                            if let Some(new_url) = new_url {
                                let op = ResourceOp::Rename(RenameFile {
                                    old_uri: uri.clone(),
                                    new_uri: new_url,
                                    options: Some(RenameFileOptions {
                                        overwrite: Some(overwrite),
                                        ignore_if_exists: None,
                                    }),
                                    annotation_id: None,
                                });
                                document_changes.push(DocumentChangeOperation::Op(op));
                            }
                        }
                        ResourceOperation::CreateFile { content } => {
                            let create_op = ResourceOp::Create(CreateFile {
                                uri: uri.clone(),
                                options: Some(CreateFileOptions {
                                    overwrite: Some(false),
                                    ignore_if_exists: Some(true),
                                }),
                                annotation_id: None,
                            });
                            document_changes.push(DocumentChangeOperation::Op(create_op));

                            if let Some(text) = content {
                                document_changes.push(DocumentChangeOperation::Edit(
                                    TextDocumentEdit {
                                        text_document: OptionalVersionedTextDocumentIdentifier {
                                            uri: uri.clone(),
                                            version: None,
                                        },
                                        edits: vec![OneOf::Left(TextEdit {
                                            range: Range {
                                                start: Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                                end: Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                            },
                                            new_text: text,
                                        })],
                                    },
                                ));
                            }
                        }
                        ResourceOperation::DeleteFile {
                            ignore_if_not_exists,
                        } => {
                            let op = ResourceOp::Delete(DeleteFile {
                                uri: uri.clone(),
                                options: Some(DeleteFileOptions {
                                    recursive: None,
                                    ignore_if_not_exists: Some(ignore_if_not_exists),
                                    annotation_id: None,
                                }),
                            });
                            document_changes.push(DocumentChangeOperation::Op(op));
                        }
                    }
                }
            }
        }

        // Flush remaining text edits
        if !current_text_edits.is_empty() {
            document_changes.push(DocumentChangeOperation::Edit(TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: None,
                },
                edits: current_text_edits
                    .into_iter()
                    .map(OneOf::Left)
                    .collect(),
            }));
        }
    }

    WorkspaceEdit {
        changes: None,
        document_changes: Some(tower_lsp::lsp_types::DocumentChanges::Operations(
            document_changes,
        )),
        change_annotations: None,
    }
}
