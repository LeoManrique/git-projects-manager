//! The latest known state of every scanned folder.
//!
//! A scan seeds its folder here after the walk and applies each repo's status
//! as it is read, and every change produces a whole, categorized
//! [`ScanResult`]. Repos keep their previous status until a newer read
//! replaces it, so a rescan never empties the screen.

use crate::domain::{PublishState, RepoStatus, ScanResult};
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Default)]
pub(super) struct ScanRegistry {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    folders: HashMap<PathBuf, FolderState>,
    /// Shared by every folder, so a revision never repeats, not even for a
    /// folder that is forgotten and scanned again.
    last_revision: u64,
}

/// One folder: every repo the latest walk found, the scan in flight and the
/// revision of the state.
struct FolderState {
    /// Keyed by repo path. `None` until the repo's first check lands.
    repos: HashMap<String, Option<Reading>>,
    pending: HashSet<String>,
    uninitialized: Vec<RepoStatus>,
    scan_started: Instant,
    started_at_ms: i64,
    is_complete: bool,
    execution_time: f64,
    revision: u64,
}

/// A repo's applied status and when the read that produced it started.
struct Reading {
    status: RepoStatus,
    started: Instant,
}

impl ScanRegistry {
    /// Start a scan of `folder`: drop the repos the walk no longer found and
    /// mark every found repo pending. Returns the first snapshot.
    pub(super) fn begin(
        &self,
        folder: &Path,
        repos: &[PathBuf],
        mut uninitialized: Vec<RepoStatus>,
        scan_started: Instant,
        started_at_ms: i64,
    ) -> ScanResult {
        let mut inner = self.inner.lock();
        let Inner { folders, last_revision } = &mut *inner;

        let mut previous = folders.remove(folder).map(|f| f.repos).unwrap_or_default();
        let repos: HashMap<String, Option<Reading>> = repos
            .iter()
            .map(|repo| {
                let path = repo.display().to_string();
                let reading = previous.remove(&path).flatten();
                (path, reading)
            })
            .collect();
        uninitialized.sort_by(|a, b| by_path_ci(&a.path, &b.path));

        let state = FolderState {
            pending: repos.keys().cloned().collect(),
            repos,
            uninitialized,
            scan_started,
            started_at_ms,
            is_complete: false,
            execution_time: 0.0,
            revision: next_revision(last_revision),
        };
        let snapshot = state.snapshot(folder);
        folders.insert(folder.to_path_buf(), state);
        snapshot
    }

    /// Apply one repo's status, read starting at `read_started`. The repo
    /// stops being pending either way, but the status is kept only if no read
    /// that started later has been applied already, so a slow read can never
    /// overwrite a fresher one.
    ///
    /// `None` when the folder is unknown (never scanned, or forgotten while
    /// the read ran) or the walk did not find the repo.
    pub(super) fn apply(
        &self,
        folder: &Path,
        status: RepoStatus,
        read_started: Instant,
    ) -> Option<ScanResult> {
        let mut inner = self.inner.lock();
        let Inner { folders, last_revision } = &mut *inner;
        let state = folders.get_mut(folder)?;
        let slot = state.repos.get_mut(&status.path)?;

        state.pending.remove(&status.path);
        if slot.as_ref().is_none_or(|r| r.started <= read_started) {
            *slot = Some(Reading { status, started: read_started });
        }
        state.revision = next_revision(last_revision);
        Some(state.snapshot(folder))
    }

    /// End the scan that started at `scan_started`. A newer scan of the same
    /// folder may have begun meanwhile; then this one leaves the state alone
    /// and the newer scan completes it.
    pub(super) fn finish(&self, folder: &Path, scan_started: Instant) -> Option<ScanResult> {
        let mut inner = self.inner.lock();
        let Inner { folders, last_revision } = &mut *inner;
        let state = folders.get_mut(folder)?;

        if state.scan_started == scan_started {
            state.is_complete = true;
            state.execution_time = scan_started.elapsed().as_secs_f64();
            state.revision = next_revision(last_revision);
        }
        Some(state.snapshot(folder))
    }

    /// Drop a folder that is no longer monitored, so its state goes with it.
    pub(super) fn forget(&self, folder: &Path) {
        self.inner.lock().folders.remove(folder);
    }
}

impl FolderState {
    fn snapshot(&self, folder: &Path) -> ScanResult {
        let mut statuses = Vec::with_capacity(self.repos.len());
        let mut checking = Vec::new();
        for (path, reading) in &self.repos {
            match reading {
                Some(r) => statuses.push(r.status.clone()),
                None => checking.push(RepoStatus::unchecked(path.clone())),
            }
        }
        checking.sort_by(|a, b| by_path_ci(&a.path, &b.path));
        let mut pending: Vec<String> = self.pending.iter().cloned().collect();
        pending.sort_by(|a, b| by_path_ci(a, b));

        let mut result = ScanResult {
            scanned_path: folder.display().to_string(),
            total_repositories: self.repos.len(),
            started_at_ms: self.started_at_ms,
            revision: self.revision,
            pending,
            checking,
            is_complete: self.is_complete,
            uninitialized: self.uninitialized.clone(),
            execution_time: self.execution_time,
            ..ScanResult::default()
        };
        categorize(&mut result, statuses);
        result
    }
}

fn next_revision(last_revision: &mut u64) -> u64 {
    *last_revision += 1;
    *last_revision
}

/// Case-insensitive ordering of repo paths, shared by every category so the
/// frontends render a stable A–Z list grouped by parent directory.
///
/// The case-sensitive tie-break makes this a total order. Without it two
/// paths differing only in case compare equal and the stable sort falls back
/// to the input order, which comes from a hash map here and from readdir
/// order before it.
fn by_path_ci(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}

/// Sort `statuses` and push each into its buckets of `result`. Each bucket
/// keeps the sorted order because the loop pushes in sequence.
fn categorize(result: &mut ScanResult, mut statuses: Vec<RepoStatus>) {
    statuses.sort_by(|a, b| by_path_ci(&a.path, &b.path));

    // Only real git repos reach this loop; uninitialized folders are kept in
    // their own list and never considered for the publish-state overlays.
    for status in statuses {
        // Unpublished and Remote Not Found are mutually-exclusive overlays:
        // a repo in either also lands in one of the exclusive buckets below.
        // Errored repos are excluded (their remote state is unknown).
        if !status.has_error {
            match status.publish_state {
                PublishState::Unpublished => result.unpublished.push(status.clone()),
                PublishState::RemoteNotFound => result.remote_not_found.push(status.clone()),
                PublishState::Published => {}
            }

            // A third overlay, on the same terms: the repo still lands in an
            // exclusive bucket below (almost always Clean, since an unknown
            // count cannot place it anywhere else), and this says the bucket
            // is not the whole story.
            if status.remote_state_unknown {
                result.remote_state_unknown.push(status.clone());
            }
        }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const FOLDER: &str = "/projects";

    fn folder() -> &'static Path {
        Path::new(FOLDER)
    }

    fn repo(name: &str) -> PathBuf {
        folder().join(name)
    }

    fn path_of(name: &str) -> String {
        repo(name).display().to_string()
    }

    fn begin(registry: &ScanRegistry, names: &[&str]) -> (ScanResult, Instant) {
        let repos: Vec<PathBuf> = names.iter().map(|n| repo(n)).collect();
        let started = Instant::now();
        (registry.begin(folder(), &repos, Vec::new(), started, 0), started)
    }

    fn with_changes(name: &str) -> RepoStatus {
        RepoStatus { has_changes: Some(true), ..RepoStatus::unchecked(path_of(name)) }
    }

    fn clean(name: &str) -> RepoStatus {
        RepoStatus::unchecked(path_of(name))
    }

    fn paths(statuses: &[RepoStatus]) -> Vec<String> {
        statuses.iter().map(|s| s.path.clone()).collect()
    }

    #[test]
    fn a_new_repo_is_checking_until_its_status_lands() {
        let registry = ScanRegistry::default();

        let (first, _) = begin(&registry, &["a"]);
        assert_eq!(paths(&first.checking), vec![path_of("a")]);
        assert_eq!(first.pending, vec![path_of("a")]);
        assert_eq!(first.total_repositories, 1);

        let applied = registry.apply(folder(), with_changes("a"), Instant::now()).unwrap();
        assert!(applied.checking.is_empty());
        assert!(applied.pending.is_empty());
        assert_eq!(paths(&applied.with_changes), vec![path_of("a")]);
    }

    #[test]
    fn a_rescan_keeps_previous_statuses_while_pending() {
        let registry = ScanRegistry::default();
        begin(&registry, &["a"]);
        registry.apply(folder(), with_changes("a"), Instant::now());

        let (rescan, _) = begin(&registry, &["a"]);

        assert!(rescan.checking.is_empty());
        assert_eq!(rescan.pending, vec![path_of("a")]);
        assert_eq!(paths(&rescan.with_changes), vec![path_of("a")]);
    }

    #[test]
    fn a_rescan_drops_repos_no_longer_on_disk() {
        let registry = ScanRegistry::default();
        begin(&registry, &["a", "b"]);
        registry.apply(folder(), clean("a"), Instant::now());
        registry.apply(folder(), clean("b"), Instant::now());

        let (rescan, _) = begin(&registry, &["a"]);

        assert_eq!(rescan.total_repositories, 1);
        assert_eq!(paths(&rescan.clean), vec![path_of("a")]);
    }

    #[test]
    fn an_older_read_does_not_overwrite_a_newer_one() {
        let registry = ScanRegistry::default();
        begin(&registry, &["a"]);
        let older = Instant::now();
        let newer = older + Duration::from_millis(1);

        registry.apply(folder(), clean("a"), newer);
        let late = registry.apply(folder(), with_changes("a"), older).unwrap();

        assert_eq!(paths(&late.clean), vec![path_of("a")]);
        assert!(late.with_changes.is_empty());
    }

    #[test]
    fn every_change_increases_the_revision() {
        let registry = ScanRegistry::default();

        let (first, started) = begin(&registry, &["a"]);
        let applied = registry.apply(folder(), clean("a"), Instant::now()).unwrap();
        let finished = registry.finish(folder(), started).unwrap();

        assert!(first.revision < applied.revision);
        assert!(applied.revision < finished.revision);
    }

    #[test]
    fn finish_completes_only_the_latest_scan() {
        let registry = ScanRegistry::default();
        let (_, superseded) = begin(&registry, &["a"]);
        let (_, latest) = begin(&registry, &["a"]);

        assert!(!registry.finish(folder(), superseded).unwrap().is_complete);
        assert!(registry.finish(folder(), latest).unwrap().is_complete);
    }

    #[test]
    fn a_forgotten_folder_ignores_late_statuses() {
        let registry = ScanRegistry::default();
        begin(&registry, &["a"]);

        registry.forget(folder());

        assert!(registry.apply(folder(), clean("a"), Instant::now()).is_none());
    }

    #[test]
    fn a_status_for_a_repo_the_walk_did_not_find_is_ignored() {
        let registry = ScanRegistry::default();
        begin(&registry, &["a"]);

        assert!(registry.apply(folder(), clean("b"), Instant::now()).is_none());
    }
}
