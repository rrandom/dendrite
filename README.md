# Dendrite

Dendrite is a Markdown Based Personal Knowledge Server.

> **Note**: Dendrite is inspired by [Dendron](https://www.dendron.so/), but designed as an editor-agnostic semantic engine that can be used by multiple clients (VS Code, CLI, Web, Desktop, etc.).

## Prerequisites

- Rust (latest stable)
- Node.js and pnpm
- VS Code (for extension development)

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
