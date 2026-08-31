//! Guards against a repository being hidden by the name of its own folder.
//!
//! `EXCLUDED_DIRS` exists to prune build output and dependency trees, and the
//! check used to run before the walk ever asked whether the directory was a
//! repository. So a repo the user happened to name `build`, `dist`, `packages`
//! or `public` was pruned on its name alone: no entry, no error, no way to
//! notice — the one failure mode this app exists to prevent.
//!
//! The pruning itself still has to work, and the vendored-dependency rule
//! (`NESTED_EXCLUDED_DIRS`) still has to win inside a repo, so both are
//! asserted here too.

use gpm_core::domain::scanner::Scanner;
use std::path::{Path, PathBuf};

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_repo(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    git(path, &["init", "-q"]);
    std::fs::write(path.join("f.txt"), "x").unwrap();
    git(path, &["add", "."]);
    git(
        path,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "i",
        ],
    );
}

fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("gpm-names-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Every path the scan reported, in any bucket.
fn scanned_paths(root: &Path) -> Vec<String> {
    let result = Scanner::new().scan_folder(root, true);
    let mut paths: Vec<String> = result
        .clean
        .iter()
        .chain(&result.with_changes)
        .chain(&result.with_unpushed)
        .chain(&result.with_unpulled)
        .chain(&result.errors)
        .map(|s| s.path.clone())
        .collect();
    paths.sort();
    paths
}

#[test]
fn a_repo_named_after_a_build_directory_is_still_found() {
    let root = fixture("excluded");

    // One repo per excluded name that a human might plausibly pick.
    let names = ["build", "dist", "packages", "public", "bin", "gen", "out"];
    for name in names {
        init_repo(&root.join(name));
    }

    let found = scanned_paths(&root);
    for name in names {
        let expected = root.join(name).display().to_string();
        assert!(
            found.contains(&expected),
            "repo named `{name}` was pruned by its own folder name; found: {found:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_build_directory_that_is_not_a_repo_is_still_pruned() {
    let root = fixture("pruned");
    init_repo(&root.join("project"));

    // A real build output: not a repo, but with a repo buried inside it that
    // must stay invisible.
    let output = root.join("project").join("dist");
    std::fs::create_dir_all(&output).unwrap();
    init_repo(&output.join("vendored-thing"));

    let found = scanned_paths(&root);
    let leaked = output.join("vendored-thing").display().to_string();
    assert!(
        !found.contains(&leaked),
        "a repo inside a build output leaked into the scan: {found:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_vendored_repo_under_lib_stays_hidden() {
    let root = fixture("vendored");
    let project = root.join("project");
    init_repo(&project);

    // `lib` is only excluded *inside* a repo, and the rule holds even when the
    // directory is itself a repo — a repo at <repo>/lib is vendored.
    init_repo(&project.join("lib"));
    init_repo(&project.join("lib").join("dependency"));

    let found = scanned_paths(&root);
    for hidden in ["lib", "lib/dependency"] {
        let path = project.join(hidden).display().to_string();
        assert!(
            !found.contains(&path),
            "vendored `{hidden}` should stay hidden; found: {found:?}"
        );
    }
    assert!(found.contains(&project.display().to_string()));

    let _ = std::fs::remove_dir_all(&root);
}
