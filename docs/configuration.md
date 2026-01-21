# Configuration Guide

Dendrite allows you to configure its behavior through two primary mechanisms.

1.  **Project-Level Configuration** (`dendrite.yaml`): Controls the core behavior of your workspace, including multi-vault setups and file ignore patterns.
2.  **Editor-Level Configuration** (VS Code Settings): Controls the behavior of the Language Server Protocol (LSP) integration, such as caching, logging, and undo history.

---

## 1. Project-Level Configuration (`dendrite.yaml`)

Place a file named `dendrite.yaml` at the root of your workspace to active these settings. If no file is present, Dendrite defaults to a single "main" vault at the root directory.

### Structure

```yaml
workspace:
  # Friendly name for your workspace
  name: "My Knowledge Base"
  
  # List of vaults (Multi-Vault Setup)
  vaults:
    # Requires at least one vault
    - name: "main" 
      path: "." # Path relative to workspace root
    
    # Optional additional vaults
    - name: "archive" 
      path: "./archive"
      
  # Glob patterns to ignore during indexing
  ignorePatterns:
    - "**/.git/**"
    - "**/node_modules/**"
    - "**/.DS_Store"

semantic:
  # The semantic model to use (currently supports "Dendron")
  model: "Dendron"
```

### Key Concepts

*   **Vaults**: Dendrite supports managing multiple physical directories ("vaults") under a single logical workspace. This allows you to split your notes (e.g., `work`, `personal`, `archive`) while maintaining unified linking (e.g., `[[archive.old-note]]`).
*   **Ignore Patterns**: Files matching these patterns will be completely skipped by the indexer, improving startup performance.

---

## 2. Editor-Level Configuration (VS Code Settings)

These settings are managed through your editor's preferences (e.g., `.vscode/settings.json` or User Settings).

### Available Settings

| Setting ID | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `dendrite.logLevel` | `string` | `"info"` | Logging verbosity. Options: `trace`, `debug`, `info`, `warning`, `error`. |
| `dendrite.cache.enabled` | `boolean` | `true` | Enable persistent caching to speed up workspace initialization. |
| `dendrite.cache.saveInterval` | `number` | `5000` | Interval (in ms) to debounce saving the cache to disk after changes. |
| `dendrite.mutationHistoryLimit` | `number` | `5` | Maximum number of undo steps stored for hierarchy operations. |

### Example `.vscode/settings.json`

```json
{
  "dendrite.logLevel": "debug",
  "dendrite.cache.enabled": true,
  "dendrite.cache.saveInterval": 10000
}
```
