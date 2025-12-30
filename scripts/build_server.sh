#!/bin/bash
# 1. Build Rust
cargo build

# 3. Create server directory
mkdir -p clients/vscode/server

# 4. Copy binary file (adjust extension based on OS)
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    cp target/debug/dendrite.exe clients/vscode/server/dendrite-server.exe
else
    cp target/debug/dendrite clients/vscode/server/dendrite-server
fi

