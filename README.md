# Dendrite

Dendrite is a Markdown-based multi-model semantic engine for Personal Knowledge Management (PKM).

> [!NOTE]
> While inspired by **Dendron**, Dendrite is built on a generalized **`SemanticModel`** abstraction. This allows it to support various ecosystems (Obsidian, Logseq, Dendron, etc.) through a single, editor-agnostic engine.

## Core Philosophy: The Semantic Model

Unlike traditional Markdown tools that are hardcoded to a specific file structure, Dendrite uses a **pluggable Semantic Model**. 

Each knowledge ecosystem (Obsidian, Dendron, Logseq) has its own "logic" for:
- **Identity**: How a file maps to a Note Key.
- **Hierarchy**: How parent-child relationships are formed (folders vs. naming conventions).
- **References**: How links are resolved and formatted.

Dendrite abstracts these behaviors, allowing the same core engine to power different workflows seamlessly. 

It is designed for speed: featuring a **two-tier persistent cache** and a **debounced background saver**, ensuring near-instant startup even for massive knowledge bases.

## Prerequisites

- Rust (latest stable)
- Node.js and pnpm
- VS Code (for extension development)

## Documentation

- [Configuration Guide](docs/configuration.md): Learn how to configure `dendrite.yaml` and VS Code settings.

## Development

### Install Dependencies

```bash
# Install Rust dependencies (handled automatically)
cargo build

# Install Node.js dependencies
pnpm install
```

### Build

**Development (VS Code):**
1. Open the project in VS Code
2. Press `F5` to launch Extension Development Host
3. The extension will automatically build before launching

**Manual Build:**
```bash
# Build Rust server (debug mode)
cargo build

# Build VS Code extension
pnpm -C clients/vscode compile
```

**Release Build:**
```bash
# Sync version, build server (release), and build extension
npm run release
```

## Project Structure

```
dendrite/
├── crates/          # Rust code (Cargo workspace)
│   ├── dendrite-core/
│   └── dendrite-lsp/
├── clients/         # Clients
│   └── vscode/      # VS Code extension
└── scripts/         # Build scripts
```

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

For commercial licensing options, please contact the project maintainers.
