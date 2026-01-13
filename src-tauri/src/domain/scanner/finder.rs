use crate::infrastructure::{exclude::EXCLUDED_DIRS, git::GitOperations};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::{DirEntry, WalkDir};

/// Responsible for finding git repositories in a directory tree
pub struct RepositoryFinder {
    cancel_flag: Arc<AtomicBool>,
}

impl RepositoryFinder {
    pub fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self { cancel_flag }
    }

    /// Find all git repositories under the given root path
    pub fn find_repositories(&self, root: &Path) -> Vec<PathBuf> {
        let mut repositories = Vec::new();

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| self.should_visit(e));

        for entry in walker {
            if self.cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            if let Ok(entry) = entry {
                if GitOperations::is_git_repo(entry.path()) {
                    repositories.push(entry.path().to_path_buf());
                }
            }
        }

        repositories
    }

    /// Determine if a directory entry should be visited during traversal
    fn should_visit(&self, entry: &DirEntry) -> bool {
        if !entry.file_type().is_dir() {
            return false;
        }

        let name = entry.file_name().to_string_lossy();

        // Skip excluded directories
        if EXCLUDED_DIRS.contains(&name.as_ref()) {
            return false;
        }

        // Skip .git directories (we've already found the repo)
        if name == ".git" {
            return false;
        }

        true
    }
}
