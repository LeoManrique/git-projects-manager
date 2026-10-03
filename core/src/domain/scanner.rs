mod finder;
mod registry;
mod remote_check;
mod status_checker;
mod uninitialized;

use crate::domain::ScanResult;
use rayon::prelude::*;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Instant;

use finder::RepositoryFinder;
use registry::ScanRegistry;
use remote_check::{RemoteCheckCtx, REMOTE_CHECK_TTL_SECS};
use status_checker::StatusChecker;
use uninitialized::UninitializedDetector;

/// Thread pool the per-repo status checks run on.
///
/// Rayon's global pool has one thread per logical CPU, which is the right size
/// for CPU-bound work. A status check is the opposite: nearly all of its wall
/// time is a `git fetch` blocked on DNS and TLS, so a CPU-sized pool turns N
/// repos into N/cpus serialized network round-trips. Oversubscribing lets them
/// overlap. Kept separate from the global pool so no other rayon user inherits
/// a thread count sized for blocking I/O.
static SCAN_POOL: LazyLock<Option<rayon::ThreadPool>> = LazyLock::new(|| {
    let cpus = std::thread::available_parallelism().map_or(8, std::num::NonZeroUsize::get);
    rayon::ThreadPoolBuilder::new()
        .num_threads((cpus * 4).clamp(8, 32))
        .thread_name(|i| format!("gpm-scan-{i}"))
        .build()
        .map_err(|e| tracing::warn!(?e, "falling back to the global rayon pool"))
        .ok()
});

/// Main scanner that orchestrates repository finding and status checking
///
/// Cancellation is deliberately absent. An earlier `cancel()` flag was polled
/// only by the directory walk, so it stopped the cheap half of a scan and left
/// every `git fetch` running, and it produced a `ScanResult` indistinguishable
/// from a complete one — which the frontends then stored as authoritative.
/// Supporting cancel properly means polling in the status loop and marking the
/// result partial; until that exists, offering the entry point is a promise the
/// scanner cannot keep.
#[derive(Default)]
pub struct Scanner {
    registry: ScanRegistry,
}

impl Scanner {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan a folder for git repositories and check their status
    ///
    /// `detect_uninitialized` is the monitored folder's own setting: a folder
    /// that is not a projects folder reports no Uninitialized entries, because
    /// there every ordinary subdirectory would be one.
    #[must_use]
    pub fn scan_folder(
        &self,
        path: &Path,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> ScanResult {
        let scan_started = Instant::now();
        let started_at_ms = chrono::Utc::now().timestamp_millis();
        // Paired with "scan finished" below: a start with no finish is a scan
        // that hung, which is otherwise indistinguishable from a slow one.
        tracing::info!(folder = %path.display(), only_local_checks, "scan started");

        // Find all git repositories
        let repositories = RepositoryFinder::find_repositories(path);

        // Find uninitialized project folders. Skipped entirely when the folder
        // is not a projects folder — it is a second full walk of the tree, so
        // not asking is also the cheaper answer.
        let uninitialized_folders = if detect_uninitialized {
            UninitializedDetector::find(&repositories)
        } else {
            Vec::new()
        };

        let first = self.registry.begin(
            path,
            &repositories,
            uninitialized_folders,
            scan_started,
            started_at_ms,
        );

        // Detecting a deleted remote requires a network fetch, so the debounce
        // cache is only loaded for online scans; local-only scans stay offline.
        let remote_ctx =
            (!only_local_checks).then(|| RemoteCheckCtx::load(REMOTE_CHECK_TTL_SECS));

        // Check status of all repositories in parallel
        let check_all = || {
            repositories.par_iter().for_each(|repo_path| {
                let read_started = Instant::now();
                let status =
                    StatusChecker::check(repo_path, only_local_checks, remote_ctx.as_ref());
                self.registry.apply(path, status, read_started);
            });
        };
        match SCAN_POOL.as_ref() {
            Some(pool) => pool.install(check_all),
            None => check_all(),
        }

        if let Some(ctx) = &remote_ctx {
            ctx.persist();
        }

        // `None` only when the folder was forgotten mid-scan, and no frontend
        // keeps the result of a folder it no longer monitors.
        let result = self.registry.finish(path, scan_started).unwrap_or(first);
        tracing::info!(
            folder = %path.display(),
            repos = result.total_repositories,
            errors = result.errors.len(),
            remote_state_unknown = result.remote_state_unknown.len(),
            elapsed_s = result.execution_time,
            "scan finished"
        );
        result
    }

    /// Drop everything known about a folder that is no longer monitored.
    pub fn forget(&self, path: &Path) {
        self.registry.forget(path);
    }
}
