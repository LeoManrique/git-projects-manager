use crate::infrastructure::{
    ignore_patterns::{EXCLUDED_DIRS, NESTED_EXCLUDED_DIRS},
    git::GitOperations,
};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Responsible for finding git repositories in a directory tree
pub struct RepositoryFinder;

impl RepositoryFinder {
    /// Find all git repositories under the given root path.
    ///
    /// Uses manual iteration with `skip_current_dir()` so that once a repo
    /// is discovered, common dependency directories (lib, `third_party`, etc.)
    /// inside it are skipped entirely — reducing I/O and avoiding false
    /// positives from vendored/cloned nested repos.
    pub fn find_repositories(root: &std::path::Path) -> Vec<PathBuf> {
        let mut repositories: Vec<PathBuf> = Vec::new();

        let mut iter = WalkDir::new(root).follow_links(false).into_iter();

        while let Some(result) = iter.next() {
            let Ok(entry) = result else {
                continue;
            };

            if !entry.file_type().is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy();

            // `.git` and hidden directories are never project roots, so they
            // are pruned before anything more expensive runs. This also covers
            // the dot-named entries in EXCLUDED_DIRS (.cache, .vscode, …).
            if name == ".git" || name.starts_with('.') {
                iter.skip_current_dir();
                continue;
            }

            // Asked before the excluded-name check: a repo the user named
            // `build`, `dist`, `packages`, `public`, `bin` or `gen` is a repo,
            // and pruning it on its name alone made it silently invisible —
            // no entry, no error. Costs one extra `stat` per pruned subtree
            // root, since the prune below still ends the descent.
            let is_repo = GitOperations::is_git_repo(entry.path());

            // Skip always-excluded dirs (node_modules, build, target, etc.)
            if !is_repo && EXCLUDED_DIRS.contains(name.as_ref()) {
                iter.skip_current_dir();
                continue;
            }

            // If we're inside a discovered repo, skip nested dependency dirs.
            // This prevents descending into lib/, third_party/, external/, etc.
            // where cloned/vendored repos with their own .git would be found.
            // Applies even when the directory is itself a repo: a repo at
            // <repo>/lib is vendored by definition, which is what this hides.
            if NESTED_EXCLUDED_DIRS.contains(name.as_ref())
                && repositories.iter().any(|r| entry.path().starts_with(r))
            {
                iter.skip_current_dir();
                continue;
            }

            if is_repo {
                repositories.push(entry.path().to_path_buf());
            }
        }

        repositories
    }
}
