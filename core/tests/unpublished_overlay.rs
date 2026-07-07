//! Verifies the Unpublished overlay: a repo with no remote appears in the
//! `unpublished` bucket *in addition to* its primary status bucket, while a
//! repo that has a remote never does.

use gpm_core::domain::scanner::Scanner;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a git command in `dir`, panicking on failure (test-only helper).
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should be installed");
    assert!(
        status.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

/// Initialize a repo at `path` with one commit. Identity is passed per-command
/// so the test does not depend on the machine's global git config.
fn init_repo_with_commit(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    git(path, &["init", "-q"]);
    std::fs::write(path.join("README.md"), "hello").unwrap();
    git(path, &["add", "."]);
    git(
        path,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@test",
            "commit",
            "-q",
            "-m",
            "init",
        ],
    );
}

fn contains(repos: &[gpm_core::domain::RepoStatus], path: &Path) -> bool {
    let target = path.display().to_string();
    repos.iter().any(|r| r.path == target)
}

#[test]
fn unpublished_repo_appears_in_both_buckets() {
    // Unique scratch dir for this test run.
    let base: PathBuf =
        std::env::temp_dir().join(format!("gpm-unpublished-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);

    // Repo A: uncommitted changes, no remote → changes + unpublished.
    let unpublished = base.join("uncommitted-unpublished");
    init_repo_with_commit(&unpublished);
    std::fs::write(unpublished.join("dirty.txt"), "wip").unwrap();

    // Repo B: has a remote, clean tree → clean, NOT unpublished.
    let published = base.join("published-clean");
    init_repo_with_commit(&published);
    git(
        &published,
        &["remote", "add", "origin", "https://example.com/x.git"],
    );

    // only_local_checks = true keeps the scan offline; has_remote is local.
    let result = Scanner::new().scan_folder(&base, true);

    assert!(
        contains(&result.with_changes, &unpublished),
        "no-remote repo with edits should be in with_changes"
    );
    assert!(
        contains(&result.unpublished, &unpublished),
        "no-remote repo should also be in unpublished (the overlay)"
    );
    assert!(
        contains(&result.clean, &published),
        "repo with a remote and clean tree should be in clean"
    );
    assert!(
        !contains(&result.unpublished, &published),
        "repo with a remote must never be in unpublished"
    );

    std::fs::remove_dir_all(&base).unwrap();
}
