use crate::domain::RepoStatus;
use crate::infrastructure::exclude::EXCLUDED_DIRS;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Responsible for detecting uninitialized project folders
/// (directories that contain files but are not git repositories)
pub struct UninitializedDetector;

impl UninitializedDetector {
    /// Find uninitialized folders that are siblings of git repositories
    pub fn find(git_repos: &[PathBuf]) -> Vec<RepoStatus> {
        let mut uninitialized = Vec::new();
        let mut checked_dirs: HashSet<PathBuf> = HashSet::new();

        // Collect all parent directories that contain git repos
        let parent_dirs: HashSet<PathBuf> = git_repos
            .iter()
            .filter_map(|repo| repo.parent().map(|p| p.to_path_buf()))
            .collect();

        // For each parent directory, check siblings of git repos
        for parent_dir in &parent_dirs {
            Self::check_directory_siblings(
                parent_dir,
                git_repos,
                &mut uninitialized,
                &mut checked_dirs,
            );
        }

        uninitialized
    }

    /// Check sibling directories of git repos in a parent directory
    fn check_directory_siblings(
        parent_dir: &Path,
        git_repos: &[PathBuf],
        uninitialized: &mut Vec<RepoStatus>,
        checked_dirs: &mut HashSet<PathBuf>,
    ) {
        let entries = match std::fs::read_dir(parent_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if !Self::should_check_directory(&path, git_repos, checked_dirs) {
                continue;
            }

            checked_dirs.insert(path.clone());
            Self::find_recursive(&path, uninitialized, checked_dirs);
        }
    }

    /// Determine if a directory should be checked for uninitialized projects
    fn should_check_directory(
        path: &Path,
        git_repos: &[PathBuf],
        checked_dirs: &HashSet<PathBuf>,
    ) -> bool {
        // Only consider directories
        if !path.is_dir() {
            return false;
        }

        // Skip if already checked
        if checked_dirs.contains(path) {
            return false;
        }

        let name = path.file_name().unwrap_or_default().to_string_lossy();

        // Skip excluded directories
        if EXCLUDED_DIRS.contains(&name.as_ref()) {
            return false;
        }

        // Skip hidden directories
        if name.starts_with('.') {
            return false;
        }

        // Skip if this is a git repo or contains git repos
        let is_or_contains_repo = git_repos.iter().any(|repo| repo.starts_with(path));
        !is_or_contains_repo
    }

    /// Recursively find uninitialized projects in a directory
    fn find_recursive(
        dir: &Path,
        uninitialized: &mut Vec<RepoStatus>,
        checked_dirs: &mut HashSet<PathBuf>,
    ) {
        if Self::directory_has_files(dir) {
            // This is a project folder (has files), mark as uninitialized
            uninitialized.push(RepoStatus {
                path: dir.display().to_string(),
                branch: None,
                has_changes: None,
                has_unpushed: None,
                has_unpulled: None,
                has_error: false,
                error_message: None,
            });
        } else {
            // Only has subdirectories, recurse into them
            Self::recurse_into_subdirectories(dir, uninitialized, checked_dirs);
        }
    }

    /// Recurse into subdirectories to find uninitialized projects
    fn recurse_into_subdirectories(
        dir: &Path,
        uninitialized: &mut Vec<RepoStatus>,
        checked_dirs: &mut HashSet<PathBuf>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if checked_dirs.contains(&path) {
                continue;
            }

            let name = path.file_name().unwrap_or_default().to_string_lossy();

            // Skip excluded and hidden directories
            if EXCLUDED_DIRS.contains(&name.as_ref()) || name.starts_with('.') {
                continue;
            }

            checked_dirs.insert(path.clone());
            Self::find_recursive(&path, uninitialized, checked_dirs);
        }
    }

    /// Check if a directory contains any files (not just subdirectories)
    fn directory_has_files(dir: &Path) -> bool {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .any(|entry| entry.path().is_file())
            })
            .unwrap_or(false)
    }
}
