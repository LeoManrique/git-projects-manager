//! Verifies the unpushed/unpulled half of a scan end to end, against a local
//! bare "remote" so no network is involved.
//!
//! The interesting property is ordering: the scan fetches, *then* reads the
//! tracking ref through the same long-lived `git2::Repository` handle. If that
//! handle ever served a cached ref, a repo that is behind would silently land
//! in Clean — the exact class of bug this whole pass is about. Each clone here
//! only learns it is behind because the in-scan fetch advanced the ref.

use gpm_core::domain::scanner::Scanner;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should be installed");
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn commit(dir: &Path, file: &str, body: &str) {
    std::fs::write(dir.join(file), body).unwrap();
    git(dir, &["add", "."]);
    git(
        dir,
        &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", file],
    );
}

struct Fixture {
    root: PathBuf,
    origin: PathBuf,
    /// A checkout used to publish commits to `origin` behind the clones' backs.
    publisher: PathBuf,
    /// The directory the scanner is pointed at.
    scanned: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("gpm-ahead-behind-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let origin = root.join("origin.git");
        let publisher = root.join("publisher");
        // Kept out of `scanned` so the bare repo and the publisher checkout are
        // not themselves picked up as repositories by the walk.
        let scanned = root.join("scanned");
        std::fs::create_dir_all(&scanned).unwrap();

        git(&root, &["init", "--bare", "-q", "-b", "main", origin.to_str().unwrap()]);
        git(&root, &["clone", "-q", origin.to_str().unwrap(), publisher.to_str().unwrap()]);
        commit(&publisher, "base.txt", "base");
        git(&publisher, &["push", "-q", "origin", "HEAD:refs/heads/main"]);

        Self { root, origin, publisher, scanned }
    }

    /// Clone `origin` into the scanned directory at the current remote tip.
    fn clone_into(&self, name: &str) -> PathBuf {
        let dst = self.scanned.join(name);
        git(
            &self.root,
            &["clone", "-q", self.origin.to_str().unwrap(), dst.to_str().unwrap()],
        );
        dst
    }

    /// Push a new commit to `origin` *after* the clones were made.
    fn advance_origin(&self, file: &str) {
        commit(&self.publisher, file, "more");
        git(&self.publisher, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
    }

    fn scan(&self) -> gpm_core::domain::ScanResult {
        Scanner::new().scan_folder(&self.scanned, false)
    }

    fn scan_local_only(&self) -> gpm_core::domain::ScanResult {
        Scanner::new().scan_folder(&self.scanned, true)
    }

    /// Leave a repo whose upstream ref names an object that is not there, and
    /// whose remote is unreachable so the fetch cannot repair it. Both the
    /// libgit2 walk and the `git rev-list` fallback fail on it.
    fn break_upstream(repo: &Path) {
        git(repo, &["remote", "set-url", "origin", "/nonexistent/gone.git"]);
        std::fs::write(
            repo.join(".git/refs/remotes/origin/main"),
            "0000000000000000000000000000000000000001\n",
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn paths(repos: &[gpm_core::domain::RepoStatus]) -> Vec<String> {
    repos
        .iter()
        .map(|r| Path::new(&r.path).file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_clone_the_scan_fetches_into_is_reported_as_unpulled() {
    let fx = Fixture::new("unpulled");
    fx.clone_into("behind");
    // The clone has no idea this exists until the scan's own fetch runs.
    fx.advance_origin("newer.txt");

    let result = fx.scan();

    assert_eq!(
        paths(&result.with_unpulled),
        vec!["behind"],
        "the scan's fetch must advance the tracking ref and the same repo handle must see it"
    );
    assert!(result.clean.is_empty(), "must not be reported clean: {:?}", paths(&result.clean));
}

#[test]
fn a_clone_with_local_commits_is_reported_as_unpushed() {
    let fx = Fixture::new("unpushed");
    let ahead = fx.clone_into("ahead");
    commit(&ahead, "local.txt", "local");

    let result = fx.scan();

    assert_eq!(paths(&result.with_unpushed), vec!["ahead"]);
    assert!(result.clean.is_empty(), "must not be reported clean: {:?}", paths(&result.clean));
}

#[test]
fn a_clone_in_sync_with_its_remote_is_clean() {
    let fx = Fixture::new("clean");
    fx.clone_into("synced");

    let result = fx.scan();

    assert_eq!(paths(&result.clean), vec!["synced"]);
    assert!(result.with_unpulled.is_empty());
    assert!(result.with_unpushed.is_empty());
    // It has a remote, so it must not appear in the Unpublished overlay.
    assert!(result.unpublished.is_empty());
    assert!(result.remote_not_found.is_empty());
}

#[test]
fn a_repo_whose_remote_comparison_fails_is_flagged_unknown_not_clean() {
    let fx = Fixture::new("unknown");
    let broken = fx.clone_into("broken");
    Fixture::break_upstream(&broken);

    let result = fx.scan();

    assert_eq!(
        paths(&result.remote_state_unknown),
        vec!["broken"],
        "the comparison was attempted and failed, so it must be reported as unknown"
    );
    // Still in its exclusive bucket — this is an overlay, like Unpublished.
    assert_eq!(paths(&result.clean), vec!["broken"]);
    assert!(result.errors.is_empty(), "an unanswerable remote is not a repo error");
}

#[test]
fn an_offline_folder_never_reports_unknown() {
    // The whole point of the distinction: `onlyLocalChecks` means the scan
    // never asked about the remote. That is the setting working, not a failure,
    // and must not fill the Unknown section with every repo in the folder.
    let fx = Fixture::new("offline");
    let broken = fx.clone_into("broken");
    Fixture::break_upstream(&broken);
    fx.clone_into("fine");

    let result = fx.scan_local_only();

    assert!(
        result.remote_state_unknown.is_empty(),
        "offline mode is not an unknown state: {:?}",
        paths(&result.remote_state_unknown)
    );
    assert_eq!(result.total_repositories, 2);
}

#[test]
fn a_second_scan_within_the_debounce_window_does_not_refetch() {
    // Fetching stays on the scan path, but bursts of scans (the rescan after a
    // pull, a focus rescan on the heels of the startup scan) ask the same
    // question twice. The second scan here reuses the tracking ref the first
    // one fetched, which is visible precisely because it misses a commit
    // pushed in between.
    let fx = Fixture::new("debounce");
    fx.clone_into("repo");

    let first = fx.scan();
    assert_eq!(paths(&first.clean), vec!["repo"]);

    fx.advance_origin("pushed-after.txt");
    let second = fx.scan();

    assert_eq!(
        paths(&second.clean),
        vec!["repo"],
        "the fetch is skipped inside the window, so the new commit is not seen yet"
    );
    assert!(second.with_unpulled.is_empty());
}

#[test]
fn a_shallow_clone_reports_its_commits_correctly() {
    // libgit2's graph walk has to cope with a truncated history: a shallow
    // clone's grafted commit has no reachable parents. The user's tree contains
    // shallow clones, so this is the case that must not turn into an error and
    // fall through to Clean.
    let fx = Fixture::new("shallow");
    let shallow = fx.scanned.join("shallow");
    git(
        &fx.root,
        &[
            "clone", "-q", "--depth", "1", "--no-local",
            &format!("file://{}", fx.origin.display()),
            shallow.to_str().unwrap(),
        ],
    );
    fx.advance_origin("deeper.txt");

    let result = fx.scan();

    assert_eq!(
        paths(&result.with_unpulled),
        vec!["shallow"],
        "one new remote commit, so exactly one repo is behind"
    );
    assert!(result.with_unpushed.is_empty(), "nothing was committed locally");
}

#[test]
fn a_detached_head_is_not_probed_against_a_remote() {
    let fx = Fixture::new("detached");
    let detached = fx.clone_into("detached");
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&detached)
        .output()
        .unwrap();
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    git(&detached, &["checkout", "-q", &head]);
    fx.advance_origin("unseen.txt");

    let result = fx.scan();

    // No branch means no upstream to compare against, so nothing is claimed.
    assert_eq!(paths(&result.clean), vec!["detached"]);
    assert!(result.with_unpulled.is_empty());
    assert!(result.with_unpushed.is_empty());
    assert!(result.errors.is_empty(), "detached HEAD is not an error");
}

#[test]
fn a_repo_with_no_commits_is_reported_as_unpushed_not_errored() {
    let fx = Fixture::new("unborn");
    let unborn = fx.scanned.join("unborn");
    std::fs::create_dir_all(&unborn).unwrap();
    git(&unborn, &["init", "-q", "-b", "main"]);

    let result = fx.scan();

    assert_eq!(paths(&result.with_unpushed), vec!["unborn"]);
    assert!(result.errors.is_empty(), "an unborn branch is a state, not a failure");
}

#[test]
fn a_pruned_tracking_ref_does_not_masquerade_as_clean_when_there_is_nothing_to_report() {
    // `branch.main.remote`/`merge` survive, but the tracking ref is gone. Both
    // the CLI and libgit2 refuse to resolve `@{upstream}` here, so the repo is
    // simply not probed — documented so the behavior is a decision, not a
    // surprise (see the "could not check" question in the audit).
    let fx = Fixture::new("pruned");
    let pruned = fx.clone_into("pruned");
    git(&pruned, &["update-ref", "-d", "refs/remotes/origin/main"]);

    let result = fx.scan();

    assert_eq!(paths(&result.clean), vec!["pruned"]);
    assert!(result.errors.is_empty());
}

#[test]
fn a_repo_with_no_upstream_is_never_probed_and_stays_clean() {
    let fx = Fixture::new("noupstream");
    let solo = fx.scanned.join("solo");
    std::fs::create_dir_all(&solo).unwrap();
    git(&solo, &["init", "-q", "-b", "main"]);
    commit(&solo, "only.txt", "only");

    let result = fx.scan();

    assert_eq!(paths(&result.clean), vec!["solo"]);
    assert!(result.with_unpushed.is_empty(), "no upstream means nothing to compare against");
    // No remote configured at all → Unpublished overlay.
    assert_eq!(paths(&result.unpublished), vec!["solo"]);
}
