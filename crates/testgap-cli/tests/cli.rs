use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_help_exits_0() {
    Command::cargo_bin("testgap")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("test gap"));
}

#[test]
fn test_init_creates_config() {
    let dir = TempDir::new().unwrap();
    Command::cargo_bin("testgap")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    assert!(dir.path().join(".testgap.toml").exists());
}

#[test]
fn test_init_no_force_fails_on_existing() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".testgap.toml"), "# existing").unwrap();
    Command::cargo_bin("testgap")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_analyze_no_ai_json() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        r#"
pub fn untested_public(x: i32) -> i32 {
    if x > 0 {
        if x > 10 {
            return x * 2;
        }
        return x + 1;
    }
    0
}
"#,
    )
    .unwrap();

    Command::cargo_bin("testgap")
        .unwrap()
        .args([
            "analyze",
            dir.path().to_str().unwrap(),
            "--no-ai",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_functions\""));
}

#[test]
fn test_analyze_fail_on_critical() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    // This function needs complexity >= 5 to be classified as Critical.
    // Complexity = 1 (base) + number of branching keywords.
    // 4 "if " keywords here -> complexity = 5 -> Critical.
    fs::write(
        src.join("lib.rs"),
        r#"
pub fn untested_public(x: i32) -> i32 {
    if x > 0 {
        if x > 10 {
            if x > 100 {
                if x > 1000 {
                    return x * 2;
                }
                return x + 1;
            }
            return x - 1;
        }
        return x;
    }
    0
}
"#,
    )
    .unwrap();

    Command::cargo_bin("testgap")
        .unwrap()
        .args([
            "analyze",
            dir.path().to_str().unwrap(),
            "--no-ai",
            "--fail-on-critical",
        ])
        .assert()
        .code(1);
}
