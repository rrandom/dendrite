# Dendrite

Dendrite is a Markdown Based Peronsal Knowledge Server.

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

```bash
# Build Rust server
cargo build

# Build VS Code extension
cd clients/vscode
npm run compile
cd ../..

# Copy binary to extension directory
# Windows PowerShell
.\scripts\build_server.ps1

# Linux/Mac
bash scripts/build_server.sh
```

### Run in VS Code

1. Open the project in VS Code
2. Press `F5` to launch Extension Development Host
3. The extension will automatically build before launching

## Project Structure

```
dendrite/
├── crates/          # Rust code (Cargo workspace)
│   ├── dendrite-core/
│   ├── dendrite-lsp/
│   └── dendrite-bin/
├── clients/         # Clients
│   └── vscode/      # VS Code extension
└── scripts/         # Build scripts
```

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

For commercial licensing options, please contact the project maintainers.
