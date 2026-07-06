use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Write `contents` to `path` atomically: write a sibling temp file, then
/// rename it over the target. A crash mid-save can no longer truncate the
/// store, and the two apps sharing these files never observe a partial write.
///
/// The temp name embeds the process id and a per-process counter so the
/// Tauri and `SwiftUI` apps (or two saves racing inside one app) can never
/// interleave writes through the same temp file.
///
/// # Errors
/// Returns an error if writing the temp file or renaming it over `path` fails.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let tmp = tmp_path(path)?;
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Like [`write_atomic`], but the file is created owner-readable only
/// (0600 on unix) before the rename makes it visible.
///
/// # Errors
/// Returns an error if writing the temp file, restricting its permissions,
/// or renaming it over `path` fails.
pub fn write_atomic_secret(path: &Path, contents: &str) -> Result<()> {
    let tmp = tmp_path(path)?;
    std::fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn tmp_path(path: &Path) -> Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| format!("invalid store path: {}", path.display()))?;
    let unique = format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    Ok(path.with_file_name(unique))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces_content() {
        let dir = std::env::temp_dir().join(format!("gpm-atomic-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("store.json");

        write_atomic(&target, "{\"a\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "{\"a\":1}");

        write_atomic(&target, "{\"a\":2}").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "{\"a\":2}");

        // No temp files left behind.
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tmp_names_are_unique_per_call() {
        let path = Path::new("/x/config.json");
        let a = tmp_path(path).unwrap();
        let b = tmp_path(path).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.parent(), Some(Path::new("/x")));
    }
}
