//! Process-wide resource limits the core raises at startup.

/// The soft open-files limit the core asks for. macOS refuses anything above
/// `OPEN_MAX` (10240) for `RLIMIT_NOFILE`, per the COMPATIBILITY section of
/// `man setrlimit`.
#[cfg(unix)]
const OPEN_FILES: libc::rlim_t = 10_240;

/// Raise the soft limit on open files to [`OPEN_FILES`], or to the hard limit
/// when that is lower.
///
/// A macOS app starts with a soft limit of 256 (`launchctl limit maxfiles`).
/// Each running git command holds its pipes, each open libgit2 repository its
/// pack files, and a clean's deletion walk one directory per level: a Clean All
/// over a 60-repo folder while that folder was rescanning ran out, and cleans
/// failed with "Too many open files (os error 24)". Only the soft limit moves,
/// which needs no privileges.
#[cfg(unix)]
pub fn raise_open_files_limit() {
    let limit = match open_files_limit() {
        Ok(limit) => limit,
        Err(error) => {
            tracing::warn!(%error, "could not read the open files limit");
            return;
        }
    };
    let target = OPEN_FILES.min(limit.rlim_max);
    if limit.rlim_cur >= target {
        return;
    }

    let raised = libc::rlimit {
        rlim_cur: target,
        rlim_max: limit.rlim_max,
    };
    // SAFETY: `setrlimit` only reads the struct it is given, which lives
    // across the call.
    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &raw const raised) } == 0 {
        tracing::info!(from = limit.rlim_cur, to = target, "raised the open files limit");
    } else {
        let error = std::io::Error::last_os_error();
        tracing::warn!(%error, "could not raise the open files limit");
    }
}

/// Windows has no per-process open-files limit to raise.
#[cfg(not(unix))]
pub fn raise_open_files_limit() {}

#[cfg(unix)]
fn open_files_limit() -> std::io::Result<libc::rlimit> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `getrlimit` only writes into the struct it is given, which lives
    // across the call.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &raw mut limit) } == 0 {
        Ok(limit)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn raises_the_soft_limit_to_the_target_or_the_hard_limit() {
        raise_open_files_limit();
        let limit = open_files_limit().expect("should read the limit");
        assert!(limit.rlim_cur >= OPEN_FILES.min(limit.rlim_max));
    }
}
