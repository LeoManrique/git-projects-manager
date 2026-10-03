//! Verifies the snapshots a scan streams and returns, and that a recheck
//! reads a repo again.

use gpm_core::domain::scanner::Scanner;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A temp folder holding one freshly initialized repo, removed on drop.
struct Fixture {
    base: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!("gpm-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let out = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .output()
            .expect("git should be installed");
        assert!(out.status.success(), "git init failed");
        Self { base }
    }

    fn path(&self) -> &Path {
        &self.base
    }

    fn repo(&self) -> PathBuf {
        self.base.join("repo")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

#[test]
fn a_finished_scan_is_complete_with_nothing_pending() {
    let fixture = Fixture::new("snapshot-complete");
    let before_ms = chrono::Utc::now().timestamp_millis();

    // only_local_checks = true keeps the scan offline.
    let result = Scanner::new().scan_folder(fixture.path(), true, false);

    assert!(result.is_complete);
    assert!(result.pending.is_empty());
    assert!(result.checking.is_empty());
    assert_eq!(result.total_repositories, 1);
    assert!(result.started_at_ms >= before_ms);
}

#[test]
fn a_rescan_has_a_higher_revision() {
    let fixture = Fixture::new("snapshot-revision");
    let scanner = Scanner::new();

    let first = scanner.scan_folder(fixture.path(), true, false);
    let second = scanner.scan_folder(fixture.path(), true, false);

    assert!(second.revision > first.revision);
}

#[test]
fn a_streaming_scan_shows_a_new_repo_as_checking_first() {
    let fixture = Fixture::new("snapshot-streaming");
    let mut snapshots = Vec::new();

    let result =
        Scanner::new().scan_folder_streaming(fixture.path(), true, false, |s| snapshots.push(s));

    let first = &snapshots[0];
    assert_eq!(first.checking.len(), 1);
    assert_eq!(first.pending.len(), 1);
    assert!(snapshots.iter().all(|s| !s.is_complete));
    assert!(snapshots.windows(2).all(|w| w[0].revision < w[1].revision));
    assert!(result.is_complete);
    assert!(result.revision > snapshots.last().unwrap().revision);
}

#[test]
fn a_recheck_reads_a_changed_repo_again() {
    let fixture = Fixture::new("snapshot-recheck");
    let scanner = Scanner::new();
    let first = scanner.scan_folder(fixture.path(), true, false);
    assert!(first.with_changes.is_empty());

    std::fs::write(fixture.repo().join("new.txt"), "x").unwrap();
    let rechecked = scanner.recheck_repos(fixture.path(), &[fixture.repo()], true).unwrap();

    assert_eq!(rechecked.with_changes.len(), 1);
    assert!(rechecked.revision > first.revision);
}

#[test]
fn a_recheck_of_a_folder_never_scanned_is_none() {
    let fixture = Fixture::new("snapshot-recheck-unknown");

    let rechecked = Scanner::new().recheck_repos(fixture.path(), &[fixture.repo()], true);

    assert!(rechecked.is_none());
}
