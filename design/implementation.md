# Dendrite - Implementation Details

This document covers the technical implementation of the `dendrite-core` and its sub-modules.

## 1. Module Responsibilities

### 1.1 Workspace (`workspace/`)
- **`mod.rs`**: Defines the `Workspace` (pure state) and `Vault` (orchestrator).
- **`vault.rs`**: High-level API for clients. Orchestrates `Workspace` and `FileSystem`.
- **`indexer.rs`**: Process-heavy logic for indexing, scanning, and mutating state.
- **`assembler.rs`**: Transitions raw parse results into semantically linked `Note` objects.
- **`cache.rs`**: Handles binary serialization/deserialization of the Workspace state.
- **`hierarchy.rs`**: Dynamically builds the `NoteTree` using the `SemanticModel`.
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
 
### 1.4 Mutation (`mutation/`)
- **`model.rs`**: Definitions for `EditPlan`, `Change`, and `Precondition`.
- **`structural.rs`**: Combined logic for **Rename** and **Move** operations.
- **`split.rs`**: Implementation of the "Extract to New Note" logic.
- **`hierarchy.rs`**: Specialized hierarchy-aware mutations (Planned).

### 1.5 Analysis (`analysis/`)
- **`audit.rs`**: Whole-workspace health check for broken links and invalid anchors.

---
 
## 2. Optimization Strategies

### 2.1 Two-Tier Invalidation
The `Indexer` uses a two-tier approach to minimize startup time and CPU usage:
1.  **Tier 1 (Metadata)**: Compares the file's `mtime` and `size` from the disk with the persistent cache metadata. If they match, the engine trusts the cached `Note` and skips reading/parsing entirely.
2.  **Tier 2 (Content Digest)**: If metadata mismatches (e.g., after a Git checkout), the file is read and hashed using SHA256. If the digest matches the cache, the engine skips the expensive parsing process.

### 2.2 Debounced Saving
The LSP layer features a `CacheManager` that listens for document changes. To prevent excessive disk I/O, it uses a **debounce** strategy (e.g., waiting for 5 seconds of silence) before persisting the latest state to `.dendrite/cache.bin`.

### 2.3 Tree Caching
The hierarchical tree view is expensive to compute for large vaults. The `Workspace` maintains a `tree_cache` protected by an `RwLock`. It is only invalidated when a file is added, removed, or renamed.

---

## 3. Strategy Traits

Dendrite is designed to be strategy-agnostic:

```rust
pub trait SemanticModel: Send + Sync {
    fn note_key_from_path(&self, path: &Path, content: &str) -> NoteKey;
    fn resolve_parent(&self, key: &NoteKey) -> Option<NoteKey>;
    fn resolve_display_name(&self, note: &Note) -> String;
}
```

The current implementation uses `DendronModel`, which interprets `.` as a hierarchy separator in filenames.

---

## 4. Refactoring Data Models

### 4.1 EditPlan Structure

```rust
struct EditPlan {
    refactor_kind: RefactorKind,
    edits: Vec<EditGroup>,
    preconditions: Vec<Precondition>,
    diagnostics: Vec<Diagnostic>,
    reversible: bool,
}

enum Change {
    TextEdit(TextEdit),
    ResourceOp(ResourceOperation),
}
```

### 4.2 LSP Mapping

| Core Concept | LSP Equivalent |
| :--- | :--- |
| `EditPlan` | `WorkspaceEdit` (partially) |
| `EditGroup` | `TextDocumentEdit` |
| `TextEdit` | `TextEdit` |
| `ResourceOperation` | `CreateFile`, `RenameFile`, `DeleteFile` |

### 4.3 Supported Operations

#### Rename & Move Note
1.  Identify the note via `NoteId`.
2.  Calculate the new path using `SemanticModel`.
3.  Query `Store` for all notes linking to the target (Backlinks).
4.  Generate `TextEdit`s for all inbound WikiLinks and Markdown links.
    - WikiLinks: Updated using the new Note Key.
    - Markdown Links: Updated using **relative path calculation**.
5.  Generate a `RenameFile` resource operation for the physical file.

#### Extract Selection to Note (Split)
1.  Extract text from source note based on `TextRange`.
2.  Create a new file with the extracted content.
3.  Replace the original selection with a WikiLink to the new note.

#### Workspace Audit
1.  Iterate through all links in the `Store`.
2.  Verify target existence (Broken Link check).
3.  Verify anchor existence (Invalid Header/Block check).
4.  Return an `EditPlan` containing only `Diagnostics` for client-side display.
