//! Bounded subprocess runner (perf P2: bound every background wait).
//!
//! `resilience-failure`: an unbounded `Command::wait` is how a stuck child
//! hangs the UI/CLI forever. Every background spawn in pacseek goes through
//! here: poll `try_wait()` and `kill()` on expiry so no caller blocks past
//! its budget. Returns `None` on timeout/spawn failure — callers map that to
//! their fail-open empty or install-hint error, never a panic.
//!
//! Deliberately unbounded (documented, user present with Ctrl-C):
//! interactive mutate runs with inherited stdio (`sudo pacman -S/-R/-Sy/-Syu`,
//! `makepkg -si`, `flatpak install/remove/update`) and pager/display tools.
//!
//! No `expect()`/`unwrap()`.

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

/// Local probes and queries (pacman -Q, -Qi, -Qdt, git diff, flatpak info).
pub const QUICK_TIMEOUT: Duration = Duration::from_secs(15);
/// Network-backed scans (pacman -Ss, paccache -d, flatpak search/list).
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);
/// Bulk transfers with user-visible progress (git clone/pull --ff-only).
pub const CLONE_TIMEOUT: Duration = Duration::from_secs(300);

/// Poll interval for `try_wait`. 50ms keeps timeout precision tight without
/// spin cost (20 wakeups/sec worst case, only while a child runs).
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Wait up to `timeout`, killing + reaping the child on expiry (no zombies,
/// no orphaned writers). `Some(status)` also covers immediate spawn errors
/// as `None` via the caller.
fn wait_bounded(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if start.elapsed() >= timeout {
            // Best-effort kill, then reap so no zombie remains.
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Captured-output spawn with stdin nulled (background child must never block
/// on input). `None` = spawn failure or timeout (child killed + reaped).
pub fn output_bounded(cmd: &str, args: &[&str], timeout: Duration) -> Option<Output> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let status = wait_bounded(&mut child, timeout)?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr);
    }
    Some(Output {
        status,
        stdout,
        stderr,
    })
}

/// Inherited-stdio spawn (user sees progress / password prompts).
/// `None` = spawn failure or timeout (child killed + reaped).
pub fn status_inherit_bounded(cmd: &str, args: &[&str], timeout: Duration) -> Option<ExitStatus> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?;
    wait_bounded(&mut child, timeout)
}

/// Silent spawn returning only the exit status (background probes like
/// `git fetch --quiet`, `flatpak info`). `None` = spawn failure or timeout.
pub fn status_null_bounded(cmd: &str, args: &[&str], timeout: Duration) -> Option<ExitStatus> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    wait_bounded(&mut child, timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_is_some_success() {
        let out = output_bounded("true", &[], Duration::from_secs(5)).unwrap();
        assert!(out.status.success());
    }

    #[test]
    fn false_is_some_failure() {
        let out = output_bounded("false", &[], Duration::from_secs(5)).unwrap();
        assert!(!out.status.success());
    }

    #[test]
    fn missing_binary_is_none() {
        assert!(
            output_bounded("definitely-not-a-binary-xyz", &[], Duration::from_secs(5)).is_none()
        );
        assert!(
            status_inherit_bounded("definitely-not-a-binary-xyz", &[], Duration::from_secs(5))
                .is_none()
        );
    }

    #[test]
    fn sleep_hits_timeout_fast() {
        // 30s sleep with 200ms budget must return None in well under 5s —
        // and reap the child (no hang, no zombie).
        let start = Instant::now();
        assert!(output_bounded("sleep", &["30"], Duration::from_millis(200)).is_none());
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn stdout_is_captured() {
        let out = output_bounded("echo", &["hello"], Duration::from_secs(5)).unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains("hello"));
    }

    #[test]
    fn inherit_true_is_some() {
        let st = status_inherit_bounded("true", &[], Duration::from_secs(5)).unwrap();
        assert!(st.success());
    }
}
