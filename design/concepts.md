# Dendrite - Core Concepts

This document defines the fundamental entities and terminology used in the Dendrite Engine.

## 1. Core Entities

Dendrite treats Markdown files as structured knowledge objects rather than simple text buffers.

### 1.1 Note
A **Note** is the primary unit of knowledge, typically mapped to a single `.md` file.
- **Identity**: Every note has a stable `NoteId` (UUID) that persists even if the file is renamed or moved.
- **Key**: A hierarchical identifier (e.g., `projects.dendrite`) used for navigation and tree construction.
- **Title**: Derived from YAML frontmatter or the first H1 heading.
- **Digest**: A SHA256 hash of the content used for efficient change detection.

### 1.2 Block
A **Block** is a semantic unit within a note (Paragraph, List Item, etc.) that can be uniquely addressed.
- **Explicit Markers**: Dendrite specifically indexes elements ending with a `^identifier` marker.
- **Addressability**: Blocks allow for fine-grained linking (`[[note#^id]]`) and future transclusion support.

### 1.3 Link
A **Link** represents a relationship between notes or blocks.
- **Types**: Supports WikiLinks `[[target]]` and standard Markdown links `[label](target)`.
- **Metadata**: Supports aliases `[[target|alias]]` and anchors `[[target#anchor]]`.
- **Graph**: The collection of all links forms a directed graph, enabling backlink discovery.

### 1.4 EditPlan
An **EditPlan** is a set of proposed structural changes to the workspace.
- **Atomic**: A plan represents a single logical operation (e.g., Rename).
- **Side-Effect Free**: The Refactor Engine only generates the plan; it does not apply it.
- **Safety**: Plans include **Preconditions** that the client must verify before execution.

---

## 2. Terminology

| Term | Definition |
| :--- | :--- |
| **Engine** | The core Rust-based semantic engine (LSP server). |
| **Vault** | High-level orchestrator that combines `Workspace` (state) and `FileSystem` (I/O). |
| **Workspace** | A **pure state container** holding notes, links, and hierarchy trees. No I/O knowledge. |
| **FileSystem** | An abstraction layer (Trait) for file system operations, enabling portability (Desktop/WASM). |
| **Identity Registry** | The module responsible for maintaining stable IDs for notes. |
| **Syntax Strategy** | The strategy responsible for format-specific rules (hierarchy, links, display names). |
| **Refactor Engine** | The core component responsible for calculating safe, semantic-aware structural changes (Rename, Move, etc.). |
| **Ghost / Virtual Note** | A node in the hierarchy that has children but no corresponding file on disk. |

---

## 3. Consistency Model

Dendrite follows an **Eventual Consistency** model:
1. **Single Source of Truth**: The local filesystem is the authoritative state.
2. **Derived State**: The memory index is a derived snapshot that can be rebuilt at any time.
3. **Overlay Priority**: When a document is open in an editor, the memory buffer (LSP Overlay) has higher priority than the disk content.
4. **Change Detection**: Using content digests, the Engine minimizes re-indexing work, only updating the graph when meaningful changes occur.
