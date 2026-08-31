//! The per-folder "projects folder" switch, end to end through a scan.
//!
//! Uninitialized answers "a project you forgot to `git init`". That question
//! only makes sense where every subfolder is meant to be a project: pointed at
//! a general-purpose folder it reports every ordinary directory and buries the
//! repositories that are actually there. The switch turns the question off, and
//! must not touch anything else the scan reports.

use gpm_core::domain::scanner::Scanner;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A repo plus a plain folder holding a file — the detector only inspects
/// siblings of a repo, so both are needed for it to report anything.
fn fixture(root: &Path) {
    let repo = root.join("a-repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    std::fs::write(repo.join("f.txt"), "x").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "i"]);

    let notes = root.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("todo.md"), "- x").unwrap();
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("gpm-uninit-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    fixture(&root);
    root
}

#[test]
fn a_projects_folder_reports_uninitialized_entries() {
    let root = temp_root("on");
    let result = Scanner::new().scan_folder(&root, true, true);

    let found: Vec<&str> = result.uninitialized.iter().map(|r| r.path.as_str()).collect();
    assert!(found.iter().any(|p| p.ends_with("notes")), "expected notes: {found:?}");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_folder_that_is_not_a_projects_folder_reports_none() {
    let root = temp_root("off");
    let result = Scanner::new().scan_folder(&root, true, false);

    assert!(
        result.uninitialized.is_empty(),
        "expected no uninitialized entries: {:?}",
        result.uninitialized
    );
    // Only that one question is suppressed: the repositories are still found
    // and still categorized, which is the whole point of monitoring the folder.
    assert_eq!(result.total_repositories, 1);
    assert_eq!(result.unpublished.len(), 1, "the fixture repo has no remote");

    let _ = std::fs::remove_dir_all(&root);
}
