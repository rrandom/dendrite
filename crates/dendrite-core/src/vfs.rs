use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Abstract interface for file system operations.
pub trait FileSystem: Send + Sync {
    /// Read the entire contents of a file into a string.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;

    /// Read the entire contents of a file into a byte vector.
    fn read_all(&self, path: &Path) -> std::io::Result<Vec<u8>>;

    /// Write exactly these bytes to the file.
    fn write_all(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()>;

    /// List all files with the given extension under the root directory.
    /// This should be a recursive search.
    fn list_files(&self, root: &Path, extension: &str) -> Vec<PathBuf>;

    /// Get metadata for a file.
    fn metadata(&self, path: &Path) -> std::io::Result<VfsMetadata>;
}

pub struct VfsMetadata {
    pub mtime: std::time::SystemTime,
    pub len: u64,
}

/// Standard implementation of FileSystem using std::fs and walkdir.
pub struct PhysicalFileSystem;

impl FileSystem for PhysicalFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn read_all(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write_all(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)
    }

    fn list_files(&self, root: &Path, extension: &str) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == extension {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        files
    }

    fn metadata(&self, path: &Path) -> std::io::Result<VfsMetadata> {
        let metadata = std::fs::metadata(path)?;
        Ok(VfsMetadata {
            mtime: metadata.modified()?,
            len: metadata.len(),
        })
    }
}
