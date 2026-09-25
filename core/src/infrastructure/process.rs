//! Running a child process under a wall-clock timeout.
//!
//! `Command::output()` waits forever. Every network-touching subprocess the
//! scanner spawns (`git fetch`, `gh`) therefore had an unbounded worst case:
//! git's own knobs bound a *stalled transfer*, but not a TCP connect to a
//! black-holed route (libcurl's default connect timeout is 300 s) and `gh` has
//! no equivalent knob at all. One such repo held a scan thread for minutes.

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Poll `try_wait` this often at first, doubling up to [`MAX_POLL`]. Starting
/// small keeps the common case (a fetch that returns in tens of milliseconds)
/// from paying a fixed poll-granularity tax; backing off keeps a slow command
/// from burning wakeups.
const MIN_POLL: Duration = Duration::from_millis(1);
const MAX_POLL: Duration = Duration::from_millis(25);

fn piped<T>(pipe: Option<T>) -> std::io::Result<T> {
    pipe.ok_or_else(|| std::io::Error::other("child pipe was not captured"))
}

/// After the child exits, how long its pipes may stay open. They close when
/// the child does, unless a descendant that left its process group still holds
/// them; waiting on that one would void the timeout.
const PIPE_GRACE: Duration = Duration::from_secs(2);

/// After `SIGTERM`, how long the group gets before `SIGKILL`: git removes its
/// lock files (`index.lock`, ref locks) on `SIGTERM`, but cannot on `SIGKILL`.
#[cfg(unix)]
const TERM_GRACE: Duration = Duration::from_secs(1);

/// Run `cmd` to completion, killing it if it outlives `timeout`.
///
/// The child's stdout and stderr are drained on their own threads: `try_wait`
/// never reads the pipes, so a child that fills the OS pipe buffer (64 KiB)
/// would block writing and never exit — deadlocking against the very loop
/// meant to time it out.
///
/// On Unix the child leads its own process group, and a timeout kills the
/// whole group. `git` does its network work in helpers it forks (`ssh`,
/// `git-remote-https`) that inherit its stderr; killing only `git` left them
/// holding the pipe open, so the reader threads — and this function — waited
/// on them for up to libcurl's 300 s connect timeout. Whatever escapes the
/// group is bounded by [`PIPE_GRACE`] instead.
///
/// # Errors
/// Returns an error if the process cannot be spawned or waited on, and
/// [`std::io::ErrorKind::TimedOut`] if it had to be killed.
pub fn output_with_timeout(cmd: &mut Command, timeout: Duration) -> std::io::Result<Output> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(cmd, 0);

    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = drain(piped(child.stdout.take())?);
    let stderr = drain(piped(child.stderr.take())?);

    let Some(status) = wait_until(&mut child, Instant::now() + timeout)? else {
        kill_tree(&mut child);
        // Reap it so the readers' pipes close.
        let _ = child.wait();
        return Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("timed out after {}s", timeout.as_secs()),
        ));
    };

    let pipes_closed_by = Instant::now() + PIPE_GRACE;
    Ok(Output {
        status,
        stdout: collect(&stdout, pipes_closed_by),
        stderr: collect(&stderr, pipes_closed_by),
    })
}

/// Read `pipe` to the end on its own thread; the receiver gets the bytes.
fn drain<R: Read + Send + 'static>(mut pipe: R) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    rx
}

/// What a [`drain`] thread read, or nothing if the pipe is still open at
/// `deadline` (the thread is left to finish on its own).
fn collect(rx: &Receiver<Vec<u8>>, deadline: Instant) -> Vec<u8> {
    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .unwrap_or_default()
}

/// Wait for `child` to exit, polling until `deadline`. `None` if it is still
/// running then.
fn wait_until(child: &mut Child, deadline: Instant) -> std::io::Result<Option<ExitStatus>> {
    let mut poll = MIN_POLL;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(poll);
        poll = (poll * 2).min(MAX_POLL);
    }
}

/// Stop `child` and, on Unix, every process in the group it leads: `SIGTERM`,
/// up to [`TERM_GRACE`] to exit, then `SIGKILL` for whatever is left.
fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    if let Ok(pgid) = libc::pid_t::try_from(child.id()) {
        signal_group(pgid, libc::SIGTERM);
        let _ = wait_until(child, Instant::now() + TERM_GRACE);
        // Helpers can outlive git, and git can ignore `SIGTERM`.
        signal_group(pgid, libc::SIGKILL);
    }
    // Still sent on Unix: harmless if the group kill already landed, and the
    // only kill there is if the pid did not fit a `pid_t`.
    let _ = child.kill();
}

#[cfg(unix)]
fn signal_group(pgid: libc::pid_t, signal: libc::c_int) {
    // SAFETY: `killpg` takes plain integers and touches no Rust memory. The
    // group is the one `process_group(0)` created for this child. Its id cannot
    // be reused while the group has a member, and the leader stays unreaped
    // until `SIGTERM`; if it exits within the grace period and the whole group
    // is gone, the `SIGKILL` reaches no one (`ESRCH`) unless, within that same
    // second, an unrelated process got the pid *and* made itself a group
    // leader.
    unsafe {
        libc::killpg(pgid, signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_output_of_a_fast_command() {
        let mut cmd = Command::new("echo");
        cmd.arg("hi");
        let out = output_with_timeout(&mut cmd, Duration::from_secs(5)).expect("should finish");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hi");
    }

    #[test]
    fn kills_a_command_that_outlives_the_timeout() {
        let mut cmd = Command::new("sleep");
        cmd.arg("30");
        let started = Instant::now();
        let err = output_with_timeout(&mut cmd, Duration::from_millis(200))
            .expect_err("should time out");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5), "should not wait for the child");
    }

    #[cfg(unix)]
    #[test]
    fn a_timeout_also_kills_grandchildren_holding_the_pipes() {
        // `sh` forks a `sleep` that inherits stdout/stderr, like git's
        // `git-remote-https` helper. Killing only `sh` would leave the readers
        // waiting 30 s for `sleep` to close the pipes.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 30; echo done");
        let started = Instant::now();
        let err = output_with_timeout(&mut cmd, Duration::from_millis(200))
            .expect_err("should time out");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5), "should not wait for the grandchild");
    }

    #[cfg(unix)]
    #[test]
    fn a_descendant_that_left_the_group_cannot_hold_the_call_open() {
        // perl moves itself into its own process group (out of reach of the
        // group kill) and keeps the inherited stdout open for 8 s, like a
        // detached ssh master would.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("perl -e 'setpgrp(0, 0); sleep 8' & echo hi");
        let started = Instant::now();
        let out = output_with_timeout(&mut cmd, Duration::from_secs(10)).expect("should finish");
        assert!(out.status.success());
        assert!(started.elapsed() < Duration::from_secs(5), "should stop waiting on the pipe");
    }

    #[test]
    fn drains_output_larger_than_the_pipe_buffer() {
        // 64 KiB is the usual pipe capacity; a naive try_wait loop deadlocks here.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("yes x | head -c 300000");
        let out = output_with_timeout(&mut cmd, Duration::from_secs(10)).expect("should finish");
        assert_eq!(out.stdout.len(), 300_000);
    }
}
