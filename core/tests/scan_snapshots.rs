//! Verifies the snapshot fields a blocking `scan_folder` returns: complete,
//! nothing left pending or checking, and stamped with its start.

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
