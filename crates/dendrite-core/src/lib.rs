//! Dendrite Core Library
//!
//! Core logic library containing Parser, Graph, Trie, etc.
//! No IO dependencies, pure logic only.
//!

pub mod analysis;
pub mod cache;
pub mod config;
pub mod identity;
pub mod line_map;
pub mod model;
pub mod mutation;
pub mod parser;
pub mod semantic;
pub mod store;
pub mod utils;
pub mod vfs;
pub mod workspace;

pub use config::DendriteConfig;
pub use identity::IdentityRegistry;
pub use semantic::{DendronModel, SemanticModel};
pub use utils::normalize_path_to_id;
pub use utils::slugify_heading;
pub use workspace::{DendriteEngine, Workspace};
