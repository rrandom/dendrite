use crate::semantic::DendronModel;
use crate::vfs::PhysicalFileSystem;
use crate::workspace::{DendriteEngine, Workspace};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_multi_vault_initialization() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let vault1_path = root.join("vault1");
    let vault2_path = root.join("vault2");

    std::fs::create_dir_all(&vault1_path).unwrap();
    std::fs::create_dir_all(&vault2_path).unwrap();

    // Create some notes
    std::fs::write(vault1_path.join("note1.md"), "# Note 1").unwrap();
    std::fs::write(vault2_path.join("note2.md"), "# Note 2").unwrap();

    // Configure
    let config = crate::config::DendriteConfig {
        workspace: crate::config::WorkspaceConfig {
            vaults: vec![
                crate::config::VaultConfig {
                    name: "vault1".to_string(),
                    path: vault1_path.clone(),
                },
                crate::config::VaultConfig {
                    name: "vault2".to_string(),
                    path: vault2_path.clone(),
                },
            ],
            ..crate::config::DendriteConfig::default().workspace
        },
        ..crate::config::DendriteConfig::default()
    };

    let model = Box::new(DendronModel::new(root.to_path_buf()));
    // Use PhysicalFileSystem because we are using TempDir which is real FS
    let fs = Arc::new(PhysicalFileSystem);
    let mut engine = DendriteEngine::new(Workspace::new(config, model), fs);

    engine.initialize(root.to_path_buf());

    // Verify notes are indexed with correct vault names
    let note1_full_path = vault1_path.join("note1.md");
    let note1_id = engine
        .workspace
        .store
        .note_id_by_path(&note1_full_path)
        .expect("note1 should exist");
    let note1 = engine
        .workspace
        .store
        .get_note(note1_id)
        .expect("note1 should exist");
    assert_eq!(note1.vault_name, "vault1");

    let note2_full_path = vault2_path.join("note2.md");
    let note2_id = engine
        .workspace
        .store
        .note_id_by_path(&note2_full_path)
        .expect("note2 should exist");
    let note2 = engine
        .workspace
        .store
        .get_note(note2_id)
        .expect("note2 should exist");
    assert_eq!(note2.vault_name, "vault2");
}

#[test]
fn test_vault_resolution() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let vault1_path = root.join("vault1");
    let vault2_path = root.join("vault2");

    let config = crate::config::DendriteConfig {
        workspace: crate::config::WorkspaceConfig {
            vaults: vec![
                crate::config::VaultConfig {
                    name: "vault1".to_string(),
                    path: vault1_path.clone(),
                },
                crate::config::VaultConfig {
                    name: "vault2".to_string(),
                    path: vault2_path.clone(),
                },
            ],
            ..crate::config::DendriteConfig::default().workspace
        },
        ..crate::config::DendriteConfig::default()
    };

    let model = Box::new(DendronModel::new(root.to_path_buf()));
    let workspace = Workspace::new(config, model);

    assert_eq!(
        workspace.vault_name_for_path(&vault1_path.join("foo.md")),
        Some("vault1".to_string())
    );
    assert_eq!(
        workspace.vault_name_for_path(&vault2_path.join("bar/baz.md")),
        Some("vault2".to_string())
    );
    assert_eq!(
        workspace.vault_name_for_path(&root.join("other/file.md")),
        None
    );
}
