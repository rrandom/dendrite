use super::*;
use crate::semantic::DendronModel;
use crate::vfs::PhysicalFileSystem;
use crate::workspace::Vault;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_cache_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    let fs = Arc::new(PhysicalFileSystem);

    let note_path = root.join("note1.md");
    fs::write(&note_path, "# Note 1\n\n[[note2]]").unwrap();

    let model = Box::new(DendronModel::new(root.clone()));
    let mut vault = Vault::new(Workspace::new(model), fs.clone());

    // Initial indexing
    vault.initialize(root.clone());
    let id_before = vault
        .workspace
        .store
        .note_id_by_path(&note_path)
        .unwrap()
        .clone();

    // Save cache
    let cache_path = root.join("cache.bin");
    vault.save_cache(&cache_path).expect("Failed to save cache");

    // Create new vault and load cache
    let model2 = Box::new(DendronModel::new(root.clone()));
    let mut vault2 = Vault::new(Workspace::new(model2), fs.clone());
    vault2
        .load_cache(&cache_path)
        .expect("Failed to load cache");

    let id_after = vault2
        .workspace
        .store
        .note_id_by_path(&note_path)
        .expect("Note missing after cache load");
    assert_eq!(
        id_before, *id_after,
        "NoteId should be preserved after cache load"
    );

    let note = vault2.workspace.store.get_note(id_after).unwrap();
    assert_eq!(note.links.len(), 1);
    assert_eq!(vault2.workspace.cache_metadata.len(), 1);
}

#[test]
fn test_cache_tier1_hit() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    let fs = Arc::new(PhysicalFileSystem);

    let note_path = root.join("note1.md");
    fs::write(&note_path, "# Note 1").unwrap();

    let model = Box::new(DendronModel::new(root.clone()));
    let mut vault = Vault::new(Workspace::new(model), fs.clone());

    vault.initialize(root.clone());
    let cache_path = root.join("cache.bin");
    vault.save_cache(&cache_path).unwrap();

    // Create new vault, load cache
    let model2 = Box::new(DendronModel::new(root.clone()));
    let mut vault2 = Vault::new(Workspace::new(model2), fs.clone());
    vault2.load_cache(&cache_path).unwrap();

    // Now "index" again. It should be a Tier 1 hit.
    // We can't easily measure "skip", but we can verify it doesn't fail.
    vault2.initialize(root.clone());

    assert_eq!(vault2.workspace.store.all_notes().count(), 2); // note1 + root
}

#[test]
fn test_cache_tier2_digest_match() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();
    let fs = Arc::new(PhysicalFileSystem);

    let note_path = root.join("note1.md");
    let content = "# Note 1";
    fs::write(&note_path, content).unwrap();

    let model = Box::new(DendronModel::new(root.clone()));
    let mut vault = Vault::new(Workspace::new(model), fs.clone());

    vault.initialize(root.clone());
    let cache_path = root.join("cache.bin");
    vault.save_cache(&cache_path).unwrap();

    // Manually change mtime but keep content same
    let mtime_old = fs::metadata(&note_path).unwrap().modified().unwrap();
    // Some systems have low mtime resolution, so we wait or force it
    // But actually, just writing the same content again will change mtime
    fs::write(&note_path, content).unwrap();
    let mtime_new = fs::metadata(&note_path).unwrap().modified().unwrap();
    assert_ne!(mtime_old, mtime_new);

    // Load cache in new vault
    let model2 = Box::new(DendronModel::new(root.clone()));
    let mut vault2 = Vault::new(Workspace::new(model2), fs.clone());
    vault2.load_cache(&cache_path).unwrap();

    // Index again. Tier 1 will miss (mtime), Tier 2 should hit (digest).
    vault2.initialize(root.clone());

    // Verify metadata was updated in cache_metadata to match new mtime
    let meta = vault2.workspace.cache_metadata.get(&note_path).unwrap();
    assert_eq!(meta.mtime, mtime_new);
}
