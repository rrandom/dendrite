//! Dendrite Core Library
//!
//! Core logic library containing Parser, Graph, Trie, etc.
//! No IO dependencies, pure logic only.
//!

mod config;
pub mod identity;
pub mod line_map;
pub mod model;
pub mod parser;
pub mod mutation;
pub mod semantic;
pub mod store;
pub mod utils;
pub mod vfs;
pub mod workspace;

pub use identity::IdentityRegistry;
pub use semantic::{DendronModel, SemanticModel};
pub use utils::normalize_path_to_id;
pub use utils::slugify_heading;
pub use workspace::{Vault, Workspace};
