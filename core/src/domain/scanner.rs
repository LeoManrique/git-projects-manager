mod finder;
mod registry;
mod remote_check;
mod status_checker;
mod uninitialized;

use crate::domain::ScanResult;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, LazyLock};
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

    /// [`Self::scan_folder_streaming`] without the intermediate snapshots.
    #[must_use]
    pub fn scan_folder(
        &self,
        path: &Path,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> ScanResult {
        self.scan_folder_streaming(path, only_local_checks, detect_uninitialized, |_| {})
    }

    /// Scan a folder for git repositories and check their status, passing
    /// `on_snapshot` the folder's state after the walk and again each time a
    /// repo's status lands. Blocks until every repo is checked and returns the
    /// final snapshot, which `on_snapshot` does not get.
    ///
    /// `detect_uninitialized` is the monitored folder's own setting: a folder
    /// that is not a projects folder reports no Uninitialized entries, because
    /// there every ordinary subdirectory would be one.
    pub fn scan_folder_streaming(
        &self,
        path: &Path,
        only_local_checks: bool,
        detect_uninitialized: bool,
        mut on_snapshot: impl FnMut(ScanResult),
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
        on_snapshot(first.clone());

        self.check_repos(path, &repositories, only_local_checks, on_snapshot);

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

    /// Read `repos` of `folder` again, after an action changed them. A repo
    /// that a scan in flight has queued but not started is skipped, since
    /// that scan reads it fresh anyway. Returns the folder's snapshot, `None`
    /// if the folder was never scanned.
    #[must_use]
    pub fn recheck_repos(
        &self,
        folder: &Path,
        repos: &[PathBuf],
        only_local_checks: bool,
    ) -> Option<ScanResult> {
        if !self.registry.knows(folder) {
            return None;
        }
        let unqueued: Vec<PathBuf> = repos
            .iter()
            .filter(|repo| !self.registry.is_queued(folder, repo))
            .cloned()
            .collect();
        self.check_repos(folder, &unqueued, only_local_checks, |_| {});
        self.registry.snapshot(folder)
    }

    /// Drop everything known about a folder that is no longer monitored.
    pub fn forget(&self, path: &Path) {
        self.registry.forget(path);
    }

    /// Check `repos` on the scan pool, one job each, applying every status to
    /// `folder` as it lands. Blocks until all are done; `on_snapshot` runs on
    /// this thread with each newer snapshot.
    ///
    /// The jobs are spawned, never joined inside the pool. A pool thread that
    /// waits on a join runs other queued jobs meanwhile, so one folder's scan
    /// used to finish only after another folder's slow repo it had picked up.
    /// This thread is outside the pool and simply blocks.
    fn check_repos(
        &self,
        folder: &Path,
        repos: &[PathBuf],
        only_local_checks: bool,
        mut on_snapshot: impl FnMut(ScanResult),
    ) {
        // Detecting a deleted remote requires a network fetch, so the debounce
        // cache is only loaded for online checks; local-only ones stay offline.
        let remote_ctx =
            (!only_local_checks).then(|| RemoteCheckCtx::load(REMOTE_CHECK_TTL_SECS));
        let ctx = remote_ctx.as_ref();
        let (tx, rx) = mpsc::channel::<ScanResult>();

        in_scan_pool(move |scope| {
            for repo in repos {
                let tx = tx.clone();
                scope.spawn(move |_| {
                    self.registry.start(folder, repo);
                    let read_started = Instant::now();
                    let status = StatusChecker::check(repo, only_local_checks, ctx);
                    if let Some(snapshot) = self.registry.apply(folder, status, read_started) {
                        let _ = tx.send(snapshot);
                    }
                });
            }
            // Only the jobs' senders are left, so the loop ends with the last job.
            drop(tx);
            // Two jobs can send in the opposite order they applied in. A newer
            // snapshot already holds the older one's status, so it is skipped.
            let mut last_revision = 0;
            for snapshot in rx {
                if snapshot.revision > last_revision {
                    last_revision = snapshot.revision;
                    on_snapshot(snapshot);
                }
            }
        });

        if let Some(ctx) = &remote_ctx {
            ctx.persist();
        }
    }
}

/// Run `op` on this thread with a scope whose jobs go to [`SCAN_POOL`], and
/// wait for them.
fn in_scan_pool<'scope>(op: impl FnOnce(&rayon::Scope<'scope>)) {
    match SCAN_POOL.as_ref() {
        Some(pool) => pool.in_place_scope(op),
        None => rayon::in_place_scope(op),
    }
}
