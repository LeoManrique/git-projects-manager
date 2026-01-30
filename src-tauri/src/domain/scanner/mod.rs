mod finder;
mod status_checker;
mod uninitialized;

use crate::domain::{RepoStatus, ScanResult};
use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use finder::RepositoryFinder;
use status_checker::StatusChecker;
use uninitialized::UninitializedDetector;

/// Main scanner that orchestrates repository finding and status checking
pub struct Scanner {
    cancel_flag: Arc<AtomicBool>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the scanner to cancel ongoing operations
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    /// Scan a folder for git repositories and check their status
    pub fn scan_folder(&self, path: &Path, only_local_checks: bool) -> ScanResult {
        let start_time = Instant::now();

        // Find all git repositories
        let finder = RepositoryFinder::new(Arc::clone(&self.cancel_flag));
        let repositories = finder.find_repositories(path);

        // Find uninitialized project folders
        let uninitialized_folders = UninitializedDetector::find(&repositories);

        // Check status of all repositories in parallel
        let statuses: Vec<RepoStatus> = repositories
            .par_iter()
            .map(|repo_path| StatusChecker::check(repo_path, only_local_checks))
            .collect();

        // Categorize results
        Self::categorize_results(path, statuses, uninitialized_folders, start_time)
    }

    /// Categorize repository statuses into different groups
    fn categorize_results(
        path: &Path,
        statuses: Vec<RepoStatus>,
        uninitialized_folders: Vec<RepoStatus>,
        start_time: Instant,
    ) -> ScanResult {
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
}
