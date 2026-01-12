//! Dendrite Core Library
//!
//! Core logic library containing Parser, Graph, Trie, etc.
//! No IO dependencies, pure logic only.
//!

mod config;
pub mod identity;
mod line_map;
pub mod model;
pub mod parser;
pub mod refactor;
pub mod semantic;
pub mod store;
pub mod workspace;

use std::path::Path;

pub fn normalize_path_to_id(path: &Path) -> String {
    let mut s = path.to_string_lossy().to_string();
    if std::path::MAIN_SEPARATOR == '\\' {
        s = s.replace('\\', "/");
    }
    s = s.trim_end_matches(".md").to_string();
    s
}

pub use identity::IdentityRegistry;
pub use semantic::{DendronModel, SemanticModel, WikiLinkFormat};
pub use workspace::{Vault, Workspace};
