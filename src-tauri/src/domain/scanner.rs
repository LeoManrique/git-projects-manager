use super::{RepoStatus, ScanResult};
use crate::infrastructure::{exclude::EXCLUDED_DIRS, git::GitOperations};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use walkdir::{DirEntry, WalkDir};

pub struct Scanner {
    cancel_flag: Arc<AtomicBool>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn scan_folder(&self, path: &Path) -> ScanResult {
        let start_time = Instant::now();
        let repositories = self.find_git_repositories(path);
        let uninitialized_folders = self.find_uninitialized_folders(path, &repositories);

        let statuses: Vec<RepoStatus> = repositories
            .par_iter()
            .map(|repo_path| self.check_repository_status(repo_path))
            .collect();

        let mut result = ScanResult {
            scanned_path: path.display().to_string(),
            total_repositories: statuses.len(),
            with_changes: vec![],
            with_unpushed: vec![],
            with_unpulled: vec![],
            clean: vec![],
            errors: vec![],
            uninitialized: uninitialized_folders,
            execution_time: start_time.elapsed().as_secs_f64(),
        };

        for status in statuses {
            if status.has_error {
                result.errors.push(status);
            } else if status.has_changes == Some(true) {
                result.with_changes.push(status);
            } else if status.has_unpushed == Some(true) {
                result.with_unpushed.push(status);
            } else if status.has_unpulled == Some(true) {
                result.with_unpulled.push(status);
            } else {
                result.clean.push(status);
            }
        }

        result
    }

    fn find_git_repositories(&self, root: &Path) -> Vec<PathBuf> {
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

    fn find_uninitialized_folders(&self, _root: &Path, git_repos: &[PathBuf]) -> Vec<RepoStatus> {
        use std::collections::HashSet;

        let mut uninitialized = Vec::new();
        let mut checked_dirs: HashSet<PathBuf> = HashSet::new();

        // Collect all parent directories that contain git repos
        let parent_dirs: HashSet<PathBuf> = git_repos
            .iter()
            .filter_map(|repo| repo.parent().map(|p| p.to_path_buf()))
            .collect();

        // For each parent directory, check siblings of git repos
        for parent_dir in &parent_dirs {
            if let Ok(entries) = std::fs::read_dir(parent_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Only consider directories
                    if !path.is_dir() {
                        continue;
                    }

                    // Skip if already checked
                    if checked_dirs.contains(&path) {
                        continue;
                    }

                    let name = path.file_name().unwrap_or_default().to_string_lossy();

                    // Skip excluded directories
                    if EXCLUDED_DIRS.contains(&name.as_ref()) {
                        continue;
                    }

                    // Skip hidden directories (starting with .)
                    if name.starts_with('.') {
                        continue;
                    }

                    // Skip if this is a git repo or contains git repos
                    let is_or_contains_repo = git_repos.iter().any(|repo| repo.starts_with(&path));
                    if is_or_contains_repo {
                        continue;
                    }

                    checked_dirs.insert(path.clone());

                    // Find uninitialized projects in this folder
                    self.find_uninitialized_recursive(&path, &mut uninitialized, &mut checked_dirs);
                }
            }
        }

        uninitialized
    }

    fn find_uninitialized_recursive(
        &self,
        dir: &Path,
        uninitialized: &mut Vec<RepoStatus>,
        checked_dirs: &mut std::collections::HashSet<PathBuf>,
    ) {
        // Check if directory has any files (not just subdirectories)
        let has_files = self.directory_has_files(dir);

        if has_files {
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
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if !path.is_dir() {
                        continue;
                    }

                    if checked_dirs.contains(&path) {
                        continue;
                    }

                    let name = path.file_name().unwrap_or_default().to_string_lossy();

                    // Skip excluded directories
                    if EXCLUDED_DIRS.contains(&name.as_ref()) {
                        continue;
                    }

                    // Skip hidden directories
                    if name.starts_with('.') {
                        continue;
                    }

                    checked_dirs.insert(path.clone());
                    self.find_uninitialized_recursive(&path, uninitialized, checked_dirs);
                }
            }
        }
    }

    fn directory_has_files(&self, dir: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    return true;
                }
            }
        }
        false
    }

    fn should_visit(&self, entry: &DirEntry) -> bool {
        if !entry.file_type().is_dir() {
            return false;
        }

        let name = entry.file_name().to_string_lossy();

        // Skip if it's an excluded directory
        if EXCLUDED_DIRS.contains(&name.as_ref()) {
            return false;
        }

        // Skip if we've already found a git repo here
        if name == ".git" {
            return false;
        }

        true
    }

    fn check_repository_status(&self, path: &Path) -> RepoStatus {
        let path_str = path.display().to_string();

        // Get branch
        let branch = match GitOperations::get_current_branch(path) {
            Ok(b) => Some(b),
            Err(e) => {
                return RepoStatus {
                    path: path_str,
                    branch: None,
                    has_changes: None,
                    has_unpushed: None,
                    has_unpulled: None,
                    has_error: true,
                    error_message: Some(format!("Failed to get branch: {}", e)),
                };
            }
        };

        // Check for changes
        let has_changes = match GitOperations::has_pending_changes(path) {
            Ok(c) => Some(c),
            Err(e) => {
                return RepoStatus {
                    path: path_str,
                    branch,
                    has_changes: None,
                    has_unpushed: None,
                    has_unpulled: None,
                    has_error: true,
                    error_message: Some(format!("Failed to check changes: {}", e)),
                };
            }
        };

        // Check for unpushed/unpulled commits (only if has upstream)
        let (has_unpushed, has_unpulled) = if GitOperations::has_upstream_branch(path).unwrap_or(false) {
            // Fetch from remote to get latest state
            let _ = GitOperations::fetch(path);

            let unpushed = match GitOperations::has_unpushed_commits(path) {
                Ok(u) => Some(u),
                Err(_) => None,
            };

            let unpulled = match GitOperations::has_unpulled_commits(path) {
                Ok(u) => Some(u),
                Err(_) => None,
            };

            (unpushed, unpulled)
        } else {
            (None, None)
        };

        RepoStatus {
            path: path_str,
            branch,
            has_changes,
            has_unpushed,
            has_unpulled,
            has_error: false,
            error_message: None,
        }
    }
}
