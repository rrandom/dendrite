# Dendrite - Implementation Details

This document covers the technical implementation of the `dendrite-core` and its sub-modules.

## 1. Module Responsibilities

### 1.1 Workspace (`workspace/`)
- **`mod.rs`**: Defines the `Workspace` (pure state) and `Vault` (orchestrator).
- **`vault.rs`**: High-level API for clients. Orchestrates `Workspace` and `FileSystem`.
- **`indexer.rs`**: Process-heavy logic for indexing, scanning, and mutating state.
- **`assembler.rs`**: Transitions raw parse results into semantically linked `Note` objects.
- **`hierarchy.rs`**: Dynamically builds the `NoteTree` using the `SyntaxStrategy`.
- **`queries.rs`**: implementation of read-only queries (links, backlinks).
- **`vfs.rs`**: `FileSystem` trait and concrete backends (`PhysicalFileSystem`).

### 1.2 Parser (`parser.rs`)
- **Technique**: Uses `pulldown-cmark` for event-based Markdown parsing.
- **Extraction**:
    - **WikiLinks**: Handles `[[target|alias#anchor]]` using custom event state tracking.
    - **Headings**: Captures hierarchical levels and text ranges.
    - **Blocks**: Scans for trailing `^id` markers in paragraphs and list items.
    - **Frontmatter**: Extracts YAML blocks using `serde_yaml`.

### 1.3 Store (`store.rs`)
- **Graph**: Uses adjacency lists to track `links` (outgoing) and `backlinks` (incoming).
- **Map**: maintains a mapping between `PathBuf`, `NoteId`, and `Note`.

---

## 2. Optimization Strategies

### 2.1 Content Digesting
Every note is hashed using SHA256 upon parsing. The `Workspace` compares the new digest with the stored one. If they match, the Engine skips:
1. Re-binding identity.
2. Updating the link graph.
3. Invalidating the hierarchy tree cache.

### 2.2 Tree Caching
The hierarchical tree view is expensive to compute for large vaults. The `Workspace` maintains a `tree_cache` protected by an `RwLock`. It is only invalidated when a file is added, removed, or renamed.

---

## 3. Strategy Traits

Dendrite is designed to be strategy-agnostic:

```rust
pub trait SyntaxStrategy: Send + Sync {
    fn note_key_from_path(&self, path: &Path, content: &str) -> NoteKey;
    fn resolve_parent(&self, key: &NoteKey) -> Option<NoteKey>;
    fn resolve_display_name(&self, note: &Note) -> String;
}
```

The current implementation uses `DendronStrategy`, which interprets `.` as a hierarchy separator in filenames.
