//! Conversion utilities between Core types and LSP types
//!
//! This module will contain functions to convert between Core's internal types
//! (Point, TextRange) and LSP types (Position, Range) when needed.

use dendrite_core::model::{Point, TextRange};
use std::path::PathBuf;
use tower_lsp::lsp_types::{Position, Range, Url};

/// Convert LSP Position to Core Point
/// LSP uses 0-based line and character (UTF-16 code units)
/// Core uses 0-based line and column (UTF-8 byte offsets)
pub fn lsp_position_to_point(position: Position) -> Point {
    Point {
        line: position.line as usize,
        col: position.character as usize,
    }
}

/// Convert Core Point to LSP Position
pub fn point_to_lsp_position(point: Point) -> Position {
    Position {
        line: point.line as u32,
        character: point.col as u32,
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
