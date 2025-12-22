# PowerShell build script
# 1. Build Rust
cargo build

# 3. Create server directory
New-Item -ItemType Directory -Force -Path clients/vscode/server | Out-Null

# 4. Copy binary file
Copy-Item -Path "target/debug/dendrite-bin.exe" -Destination "clients/vscode/server/dendrite-server.exe" -Force

Write-Host "Build complete! Server executable copied to clients/vscode/server/dendrite-server.exe"

