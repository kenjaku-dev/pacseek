use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_works() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("pacseek"));
}

#[test]
fn help_shows_refresh_flag() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh"));
}

#[test]
fn version_works() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pacseek"));
}

#[test]
fn repo_search_smoke() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    cmd.args(["firefox", "--source", "repo", "--limit", "2", "--no-color"])
        .assert()
        .success();
}

#[test]
fn json_output_is_valid() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    let output = cmd
        .args(["neovim", "--limit", "1", "--json", "--no-color"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json should be valid");
    assert!(v.is_array());
}

#[test]
fn regex_recall_matches_substring() {
    // Regression: --regex must find what substring search finds. The repo
    // regex path scans all DB packages (never feeds the raw pattern to
    // libalpm's literal search), so `firef.x` must match `firefox`.
    for args in [
        vec!["firefox", "--source", "repo", "--limit", "5", "--no-color"],
        vec![
            "firef.x",
            "--source",
            "repo",
            "--limit",
            "5",
            "--no-color",
            "--regex",
        ],
    ] {
        let mut cmd = Command::cargo_bin("pacseek").unwrap();
        let output = cmd.args(&args).output().unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        assert!(
            stdout.contains("firefox"),
            "args {args:?} should match firefox"
        );
    }
}

#[test]
fn help_shows_new_maintenance_flags() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--updates"))
        .stdout(predicate::str::contains("--orphans"))
        .stdout(predicate::str::contains("--stats"))
        .stdout(predicate::str::contains("--upgrade"));
}

#[test]
fn updates_lists_valid_json() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    let output = cmd.args(["--updates", "--json"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("updates json valid");
    assert!(v.is_array());
}

#[test]
fn orphans_lists_valid_json() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    let output = cmd.args(["--orphans", "--json"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("orphans json valid");
    assert!(v.is_array());
}

#[test]
fn stats_valid_json() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    let output = cmd.args(["--stats", "--json"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stats json valid");
    assert!(v.get("explicit").is_some());
    assert!(v.get("orphans").is_some());
}

#[test]
fn readonly_refuses_mutating_ops() {
    for args in [
        vec!["--readonly", "--remove", "vim"],
        vec!["--readonly", "--refresh"],
        vec!["--readonly", "--upgrade"],
    ] {
        let mut cmd = Command::cargo_bin("pacseek").unwrap();
        let output = cmd
            .args(&args)
            .env("PACSEEK_NOCONFIRM", "1")
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "args {args:?} must be refused in readonly"
        );
    }
}

fn paccache_present() -> bool {
    std::process::Command::new("which")
        .arg("paccache")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn help_shows_clean_flags() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--clean"))
        .stdout(predicate::str::contains("--keep"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn clean_dry_run_json_is_valid() {
    // Needs paccache (pacman-contrib); skip gracefully off-Arch.
    if !paccache_present() {
        return;
    }
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    let output = cmd
        .args(["--clean", "--dry-run", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("clean json valid");
    assert!(v.get("files").is_some());
    assert!(v.get("bytes").is_some());
}

#[test]
fn readonly_refuses_clean() {
    let mut cmd = Command::cargo_bin("pacseek").unwrap();
    let output = cmd
        .args(["--readonly", "--clean", "--dry-run"])
        .env("PACSEEK_NOCONFIRM", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
}
