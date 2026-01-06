use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Abstract interface for file system operations.
pub trait FileSystem: Send + Sync {
    /// Read the entire contents of a file into a string.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;

    /// List all files with the given extension under the root directory.
    /// This should be a recursive search.
    fn list_files(&self, root: &Path, extension: &str) -> Vec<PathBuf>;
}

/// Standard implementation of FileSystem using std::fs and walkdir.
pub struct PhysicalFileSystem;

impl FileSystem for PhysicalFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
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
}
