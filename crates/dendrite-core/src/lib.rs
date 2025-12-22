//! Dendrite Core Library
//!
//! Core logic library containing Parser, Graph, Trie, etc.
//! No IO dependencies, pure logic only.
//!

pub mod workspace;
pub mod store;
pub mod model;
mod config;
mod graph;
mod trie;
mod strategy;

use std::path::Path;

pub fn normalize_path_to_id(path: &Path) -> String {
    let mut s = path.to_string_lossy().to_string();
    if std::path::MAIN_SEPARATOR == '\\' {
        s = s.replace('\\', "/");
    }
    s.trim_end_matches(".md").to_string();
    s
}
