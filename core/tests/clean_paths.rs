//! Verifies `GitOperations::clean` against the two ways it used to report a
//! total failure for a repo it had partly (or fully) cleaned successfully.

use gpm_core::infrastructure::git::GitOperations;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a git command in `dir`, panicking on failure (test-only helper).
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should be installed");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A repo with one commit whose `.gitignore` ignores `ign/` and `*.log`.
fn init_repo(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    git(path, &["init", "-q"]);
    std::fs::write(path.join(".gitignore"), "ign/\n*.log\n").unwrap();
    git(path, &["add", ".gitignore"]);
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

fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("gpm-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    base
}

/// `core.quotePath` defaults to true, so `git clean -n` prints a path with any
/// non-ASCII byte C-quoted (`"caf\303\251/"`). Joining that literal onto the
/// repo root produced a path that does not exist, and the `?` on the delete
/// aborted the whole repo — the same repos failing on every single run.
#[test]
fn cleans_paths_with_non_ascii_names() {
    let repo = scratch("clean-utf8");
    init_repo(&repo);

    std::fs::create_dir_all(repo.join("café")).unwrap();
    std::fs::write(repo.join("café/ñandú.log"), "x").unwrap();
    std::fs::write(repo.join("plain.log"), "x").unwrap();

    let (files, dirs) = GitOperations::clean(&repo, &[]).expect("clean should succeed");

    assert!(!repo.join("café/ñandú.log").exists(), "café/ should be gone");
    assert!(!repo.join("plain.log").exists(), "plain.log should be gone");
    assert!(files.contains(&"plain.log".to_string()), "got files {files:?}");
    assert!(dirs.contains(&"café".to_string()), "got dirs {dirs:?}");

    std::fs::remove_dir_all(&repo).unwrap();
}

/// A build, a watcher or a language server can delete a listed path between the
/// dry run and the delete. That must not abandon the paths still to come.
#[test]
fn a_path_removed_after_the_dry_run_is_not_a_failure() {
    let repo = scratch("clean-race");
    init_repo(&repo);

    // Two ignored files; `git clean -n` lists both, but one vanishes before we
    // get to it. Names chosen so the survivor sorts after the casualty.
    std::fs::write(repo.join("a-vanishes.log"), "x").unwrap();
    std::fs::write(repo.join("b-survives.log"), "x").unwrap();
    std::fs::remove_file(repo.join("a-vanishes.log")).unwrap();

    let (files, _) = GitOperations::clean(&repo, &[]).expect("clean should succeed");

    assert!(!repo.join("b-survives.log").exists(), "the rest must still be cleaned");
    assert_eq!(files, vec!["b-survives.log".to_string()]);

    std::fs::remove_dir_all(&repo).unwrap();
}

/// Excluded patterns still win over deletion.
#[test]
fn exclude_patterns_are_preserved() {
    let repo = scratch("clean-exclude");
    init_repo(&repo);

    std::fs::write(repo.join("keep.log"), "x").unwrap();
    std::fs::write(repo.join("drop.log"), "x").unwrap();

    let (files, _) =
        GitOperations::clean(&repo, &["keep.log".to_string()]).expect("clean should succeed");

    assert!(repo.join("keep.log").exists(), "excluded path must survive");
    assert_eq!(files, vec!["drop.log".to_string()]);

    std::fs::remove_dir_all(&repo).unwrap();
}
