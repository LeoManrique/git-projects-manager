//! Verifies that repos within a category come back sorted case-insensitively
//! by absolute path, regardless of the directory walk's discovery order.

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

/// Initialize a clean repo at `path` with one commit and a remote, so it lands
/// in the `clean` bucket (remote → not in the Unpublished overlay).
fn init_clean_repo(path: &Path) {
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
    git(path, &["remote", "add", "origin", "https://example.com/x.git"]);
}

#[test]
fn repos_are_sorted_case_insensitively_by_path() {
    let base: PathBuf =
        std::env::temp_dir().join(format!("gpm-ordering-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);

    // Created in a deliberately unsorted order, with mixed case: a naive
    // case-sensitive sort would group the capitalized names first
    // (Mango, Zebra, apple, banana), so this asserts the case-insensitive order.
    for name in ["Zebra", "apple", "Mango", "banana"] {
        init_clean_repo(&base.join(name));
    }

    // only_local_checks = true keeps the scan offline.
    let result = Scanner::new().scan_folder(&base, true);

    let names: Vec<String> = result
        .clean
        .iter()
        .map(|r| Path::new(&r.path).file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        vec!["apple", "banana", "Mango", "Zebra"],
        "clean repos should be sorted case-insensitively by path"
    );

    std::fs::remove_dir_all(&base).unwrap();
}
