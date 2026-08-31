//! Guards the uninitialized-folder walk against symlinks.
//!
//! The recursion has no depth limit and remembers visited directories by their
//! literal (non-canonicalized) path, so a symlink pointing back at an ancestor
//! produced a fresh key at every level and the walk kept descending. It does
//! not hang — the OS refuses to resolve a symlink chain past its own limit
//! (`ELOOP`, ~32 links), and the failed `read_dir` ends the recursion — so the
//! damage is bounded but real: the *same* project folder gets reported once
//! per level, and each level costs another round of directory reads.

use gpm_core::domain::scanner::Scanner;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// The detector only looks at siblings of a repo, so every fixture needs one.
fn seed_repo(root: &Path) {
    let repo = root.join("a-repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    std::fs::write(repo.join("f.txt"), "x").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "i"]);
}

fn scan_with_timeout(root: &Path) -> gpm_core::domain::ScanResult {
    let root = root.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Scanner::new().scan_folder(&root, true));
    });
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .expect("scan should terminate; a symlink cycle means it did not")
}

#[test]
fn a_symlink_cycle_does_not_report_the_same_folder_repeatedly() {
    let root = std::env::temp_dir().join(format!("gpm-symlink-cycle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    seed_repo(&root);

    // `nest/` holds no files, so the walk recurses into both of its entries:
    // `inner/` (a real project folder, reported once) and `back` (a link to
    // `nest` itself). Following `back` re-reaches `inner` as
    // nest/back/inner, then nest/back/back/inner, and so on.
    let nest = root.join("nest");
    let inner = nest.join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    std::fs::write(inner.join("main.py"), "print()").unwrap();
    std::os::unix::fs::symlink(&nest, nest.join("back")).unwrap();

    let result = scan_with_timeout(&root);

    let found: Vec<&str> = result.uninitialized.iter().map(|r| r.path.as_str()).collect();
    let inner_hits = found.iter().filter(|p| p.ends_with("inner")).count();
    assert_eq!(inner_hits, 1, "one project folder, one entry: {found:?}");
    assert!(
        found.iter().all(|p| !p.contains("back")),
        "must not descend through the symlink: {found:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_symlinked_project_folder_is_not_reported_as_uninitialized() {
    // Consistency with `RepositoryFinder`, which walks with `follow_links(false)`:
    // if a symlinked tree's repos are invisible, its folders must be too, or the
    // same directory gets reported as an uninitialized project.
    let root = std::env::temp_dir().join(format!("gpm-symlink-target-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    seed_repo(&root);

    let real = root.join("elsewhere");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("main.py"), "print()").unwrap();
    std::os::unix::fs::symlink(&real, root.join("linked-project")).unwrap();

    let result = scan_with_timeout(&root);

    let found: Vec<&str> = result.uninitialized.iter().map(|r| r.path.as_str()).collect();
    assert!(
        found.iter().any(|p| p.ends_with("elsewhere")),
        "the real directory is still a project folder: {found:?}"
    );
    assert!(
        found.iter().all(|p| !p.ends_with("linked-project")),
        "the symlink must not be reported as a second one: {found:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
