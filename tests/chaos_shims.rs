//! Chaos shims (perf P5): fake `flatpak`/`paccache` binaries on PATH proving
//! fail-open + bounded behavior under garbage / exit-1 / hung subprocesses.
//!
//! Hermetic by construction: each test builds its own shim dir, prepends it
//! to PATH under a process-wide mutex, and restores PATH afterwards. This
//! file is its own test binary, so PATH mutation can never leak into the
//! lib/integration suites running in other processes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;

static PATH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Holds the PATH mutex and restores the original PATH on drop.
struct PathGuard {
    original: Option<OsString>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl PathGuard {
    fn prepend(dir: &std::path::Path) -> Self {
        let lock = PATH_LOCK.lock().unwrap();
        let original = std::env::var_os("PATH");
        let mut paths: Vec<std::path::PathBuf> = vec![dir.to_path_buf()];
        if let Some(ref o) = original {
            paths.extend(std::env::split_paths(o));
        }
        // Keep system dirs so `which`/`sh` resolve; shim dir wins.
        if !paths.iter().any(|p| p == std::path::Path::new("/usr/bin")) {
            paths.push(std::path::PathBuf::from("/usr/bin"));
        }
        if !paths.iter().any(|p| p == std::path::Path::new("/bin")) {
            paths.push(std::path::PathBuf::from("/bin"));
        }
        let joined = std::env::join_paths(paths).unwrap();
        unsafe {
            std::env::set_var("PATH", &joined);
        }
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        unsafe {
            match self.original.take() {
                Some(o) => std::env::set_var("PATH", o),
                None => std::env::remove_var("PATH"),
            }
        }
    }
}

/// Write an executable shim named `name` into a fresh tempdir.
fn shim_dir(name: &str, body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, body).unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    dir
}

const ONE_APP_JSON: &str = r#"[{"name":"ChaosApp","application_id":"org.chaos.App","version":"1.0","branch":"stable","arch":"x86_64","origin":"flathub","installation":"system"}]"#;

#[test]
fn garbage_flatpak_is_failopen_empty() {
    let dir = shim_dir(
        "flatpak",
        "#!/bin/sh\necho 'this is not json at all'\nexit 0\n",
    );
    let _guard = PathGuard::prepend(dir.path());
    // Own the memos: sibling tests share this process, so invalidate first —
    // otherwise a memo filled by another test would serve this one and the
    // shim would never execute (guard serializes, so no race).
    pacseek::sources::invalidate_flatpak_memo();
    pacseek::sources::invalidate_flatpak_list_memo();
    assert!(pacseek::sources::has_flatpak());
    let cfg = pacseek::config::Config::default();
    assert!(
        pacseek::sources::list_installed()
            .unwrap_or_default()
            .is_empty()
    );
    assert!(
        pacseek::sources::search_flatpak_with_config("firefox", 10, false, &cfg)
            .unwrap_or_default()
            .is_empty()
    );
    assert!(pacseek::sources::check_flatpak_updates_with_config(&cfg).is_empty());
}

#[test]
fn exit1_flatpak_is_failopen_empty() {
    let dir = shim_dir("flatpak", "#!/bin/sh\nexit 1\n");
    let _guard = PathGuard::prepend(dir.path());
    // Own the memos: sibling tests share this process, so invalidate first —
    // otherwise a memo filled by another test would serve this one and the
    // shim would never execute (guard serializes, so no race).
    pacseek::sources::invalidate_flatpak_memo();
    pacseek::sources::invalidate_flatpak_list_memo();
    assert!(pacseek::sources::has_flatpak());
    let cfg = pacseek::config::Config::default();
    assert!(
        pacseek::sources::list_installed()
            .unwrap_or_default()
            .is_empty()
    );
    assert!(pacseek::sources::check_flatpak_updates_with_config(&cfg).is_empty());
}

#[test]
fn slow_remote_info_is_bounded() {
    // `list` answers instantly with one app; `remote-info` hangs.
    // With a 2s per-probe budget the whole check must finish in seconds,
    // not hang, and report no updates (fail-open).
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = \"list\" ]; then echo '{ONE_APP_JSON}'; else sleep 30; fi\n"
    );
    let dir = shim_dir("flatpak", &body);
    let _guard = PathGuard::prepend(dir.path());
    pacseek::sources::invalidate_flatpak_memo();
    pacseek::sources::invalidate_flatpak_list_memo();
    let mut cfg = pacseek::config::Config::default();
    cfg.search.timeout_secs = 2;
    let start = std::time::Instant::now();
    let updates = pacseek::sources::check_flatpak_updates_with_config(&cfg);
    let elapsed = start.elapsed();
    assert!(updates.is_empty());
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "hung remote-info must be bounded, took {elapsed:?}"
    );
}

#[test]
fn garbage_paccache_preview_is_zero() {
    let dir = shim_dir(
        "paccache",
        "#!/bin/sh\necho 'total garbage output'\nexit 0\n",
    );
    let _guard = PathGuard::prepend(dir.path());
    let preview = pacseek::install::clean::preview_clean(2).unwrap();
    assert_eq!((preview.files, preview.bytes), (0, 0));
}
