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
