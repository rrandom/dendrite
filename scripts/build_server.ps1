param (
    [switch]$Release
)

# 1. Build Rust
if ($Release) {
    Write-Host "Building Release version..."
    cargo build --release
    $SourcePath = "target/release/dendrite.exe"
} else {
    Write-Host "Building Debug version..."
    cargo build
    $SourcePath = "target/debug/dendrite.exe"
}

# 3. Create server directory
New-Item -ItemType Directory -Force -Path clients/vscode/server | Out-Null

# 4. Copy binary file
if (Test-Path $SourcePath) {
    Copy-Item -Path $SourcePath -Destination "clients/vscode/server/dendrite-server.exe" -Force
    Write-Host "Build complete! Server executable copied to clients/vscode/server/dendrite-server.exe"
} else {
    Write-Error "Build failed or binary not found at $SourcePath"
    exit 1
}

