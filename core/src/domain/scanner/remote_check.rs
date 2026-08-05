use crate::infrastructure::github_cli::{self, RepoExistence};
use crate::infrastructure::remote_check_store::{RemoteCheckEntry, RemoteCheckStore};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// How long a `gh` remote-existence verdict is trusted before we re-confirm.
/// A deleted remote staying deleted is the norm, and a *re-created* remote is
/// caught immediately by a successful `git fetch` (which bypasses this path),
/// so the window only bounds redundant `gh` calls for still-gone remotes.
pub(crate) const REMOTE_CHECK_TTL_SECS: i64 = 24 * 60 * 60;

/// Shared, thread-safe debounce for `gh` remote-existence checks during one
/// scan. Loaded once, consulted per repo across the rayon pool, saved once.
pub(crate) struct RemoteCheckCtx {
    cache: Mutex<HashMap<String, RemoteCheckEntry>>,
    store: RemoteCheckStore,
    now: i64,
    ttl_secs: i64,
    dirty: AtomicBool,
}

impl RemoteCheckCtx {
    pub(crate) fn load(ttl_secs: i64) -> Self {
        let store = RemoteCheckStore::new();
        let cache = Mutex::new(store.load());
        Self {
            cache,
            store,
            now: now_epoch_secs(),
            ttl_secs,
            dirty: AtomicBool::new(false),
        }
    }

    /// Whether the repo's remote is confirmed gone. Called only after `git
    /// fetch` already reported "not found", so this just adds the `gh`
    /// confirmation, debounced by the persisted cache. Anything `gh` cannot
    /// judge (offline, non-GitHub, unauthenticated) is treated as *not* gone.
    pub(crate) fn is_remote_gone(&self, repo_path: &Path) -> bool {
        let key = repo_path.display().to_string();

        // Fresh cached verdict — skip the `gh` round-trip. The guard is dropped
        // at the end of this block, before any network call below.
        if let Some(entry) = self.lock().get(&key)
            && self.now - entry.checked_at < self.ttl_secs
        {
            return !entry.exists;
        }

        match github_cli::repo_exists_in_dir(repo_path) {
            RepoExistence::Exists => {
                self.record(key, true);
                false
            }
            RepoExistence::NotFound => {
                self.record(key, false);
                true
            }
            // Inconclusive: don't cache and don't flag.
            RepoExistence::Unknown => false,
        }
    }

    /// Persist the cache if any verdict changed this scan.
    pub(crate) fn persist(&self) {
        if self.dirty.load(Ordering::Relaxed) {
            let _ = self.store.save(&self.lock());
        }
    }

    fn record(&self, key: String, exists: bool) {
        self.lock().insert(
            key,
            RemoteCheckEntry {
                checked_at: self.now,
                exists,
            },
        );
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, RemoteCheckEntry>> {
        // Recover from a poisoned lock: the map is plain data, so a panicking
        // sibling task can't leave it in an invalid state worth aborting for.
        self.cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
