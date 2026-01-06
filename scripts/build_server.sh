#!/bin/bash
RELEASE_MODE=false

for arg in "$@"; do
  case $arg in
    --release)
      RELEASE_MODE=true
      shift
      ;;
  esac
done

# 3. Create server directory
mkdir -p clients/vscode/server

if [ "$RELEASE_MODE" = true ]; then
    echo "Building Release version..."
    cargo build --release
    SOURCE_DIR="target/release"
else
    echo "Building Debug version..."
    cargo build
    SOURCE_DIR="target/debug"
fi

# 4. Copy binary file (adjust extension based on OS)
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    SOURCE_FILE="$SOURCE_DIR/dendrite.exe"
    DEST_FILE="clients/vscode/server/dendrite-server.exe"
else
    SOURCE_FILE="$SOURCE_DIR/dendrite"
    DEST_FILE="clients/vscode/server/dendrite-server"
fi

if [ -f "$SOURCE_FILE" ]; then
    cp "$SOURCE_FILE" "$DEST_FILE"
    echo "Build complete! Server executable copied to $DEST_FILE"
else
    echo "Error: Binary not found at $SOURCE_FILE"
    exit 1
fi

