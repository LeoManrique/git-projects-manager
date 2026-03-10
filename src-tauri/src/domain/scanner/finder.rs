use crate::infrastructure::{
    exclude::{EXCLUDED_DIRS, NESTED_EXCLUDED_DIRS},
    git::GitOperations,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

/// Responsible for finding git repositories in a directory tree
pub struct RepositoryFinder {
    cancel_flag: Arc<AtomicBool>,
}

impl RepositoryFinder {
    pub fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self { cancel_flag }
    }

    /// Find all git repositories under the given root path.
    ///
    /// Uses manual iteration with `skip_current_dir()` so that once a repo
    /// is discovered, common dependency directories (lib, third_party, etc.)
    /// inside it are skipped entirely — reducing I/O and avoiding false
    /// positives from vendored/cloned nested repos.
    pub fn find_repositories(&self, root: &std::path::Path) -> Vec<PathBuf> {
        let mut repositories: Vec<PathBuf> = Vec::new();

        let mut iter = WalkDir::new(root).follow_links(false).into_iter();

        while let Some(result) = iter.next() {
            if self.cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy();

            // Skip always-excluded dirs (node_modules, build, .cache, etc.)
            if name == ".git" || EXCLUDED_DIRS.contains(name.as_ref()) {
                iter.skip_current_dir();
                continue;
            }

            // Skip hidden directories
            if name.starts_with('.') {
                iter.skip_current_dir();
                continue;
            }

            // If we're inside a discovered repo, skip nested dependency dirs.
            // This prevents descending into lib/, third_party/, external/, etc.
            // where cloned/vendored repos with their own .git would be found.
            if NESTED_EXCLUDED_DIRS.contains(name.as_ref())
                && repositories.iter().any(|r| entry.path().starts_with(r))
            {
                iter.skip_current_dir();
                continue;
            }

            if GitOperations::is_git_repo(entry.path()) {
                repositories.push(entry.path().to_path_buf());
            }
        }

        repositories
    }
}
