//! Running a child process under a wall-clock timeout.
//!
//! `Command::output()` waits forever. Every network-touching subprocess the
//! scanner spawns (`git fetch`, `gh`) therefore had an unbounded worst case:
//! git's own knobs bound a *stalled transfer*, but not a TCP connect to a
//! black-holed route (libcurl's default connect timeout is 300 s) and `gh` has
//! no equivalent knob at all. One such repo held a scan thread for minutes.

use std::io::Read;
use std::process::{Command, Output, Stdio};
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

/// Run `cmd` to completion, killing it if it outlives `timeout`.
///
/// The child's stdout and stderr are drained on their own threads: `try_wait`
/// never reads the pipes, so a child that fills the OS pipe buffer (64 KiB)
/// would block writing and never exit — deadlocking against the very loop
/// meant to time it out.
///
/// # Errors
/// Returns an error if the process cannot be spawned or waited on, and
/// [`std::io::ErrorKind::TimedOut`] if it had to be killed.
pub fn output_with_timeout(cmd: &mut Command, timeout: Duration) -> std::io::Result<Output> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdout = piped(child.stdout.take())?;
    let mut stderr = piped(child.stderr.take())?;
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + timeout;
    let mut poll = MIN_POLL;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            // Reap it so the readers' pipes close and the threads below join.
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("timed out after {}s", timeout.as_secs()),
            ));
        }
        std::thread::sleep(poll);
        poll = (poll * 2).min(MAX_POLL);
    };

    Ok(Output {
        status,
        stdout: out_reader.join().unwrap_or_default(),
        stderr: err_reader.join().unwrap_or_default(),
    })
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

    #[test]
    fn drains_output_larger_than_the_pipe_buffer() {
        // 64 KiB is the usual pipe capacity; a naive try_wait loop deadlocks here.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("yes x | head -c 300000");
        let out = output_with_timeout(&mut cmd, Duration::from_secs(10)).expect("should finish");
        assert_eq!(out.stdout.len(), 300_000);
    }
}
