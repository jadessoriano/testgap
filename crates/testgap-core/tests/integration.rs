use std::fs;
use std::path::Path;
use tempfile::TempDir;
use testgap_core::{analyze, config::TestGapConfig, TestGapError};

fn create_fixture(dir: &Path) {
    // Create src/lib.rs with functions of varying visibility/complexity
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn public_complex(x: i32) -> i32 {
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

pub fn public_simple(x: i32) -> i32 {
    x + 1
}

fn private_func(x: i32) -> i32 {
    if x > 0 {
        if x > 10 {
            if x > 100 {
                return x;
            }
            return x - 1;
        }
        return x + 1;
    }
    0
}

pub fn tested_func(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .unwrap();

    // Create tests/some_tests.rs that covers only tested_func.
    // The file name intentionally does NOT match "test_lib" to avoid
    // file-based mapping that would mark all lib.rs functions as covered.
    // The body references only tested_func so body-matching won't cover others.
    let test_dir = dir.join("tests");
    fs::create_dir_all(&test_dir).unwrap();
    fs::write(
        test_dir.join("some_tests.rs"),
        r#"
fn test_tested_func() {
    let result = tested_func(1, 2);
    assert_eq!(result, 3);
}
"#,
    )
    .unwrap();
}

#[tokio::test]
async fn test_analyze_finds_gaps() {
    let dir = TempDir::new().unwrap();
    create_fixture(dir.path());

    let mut config = TestGapConfig::default();
    config.ai.enabled = false;

    let report = analyze(dir.path(), &config)
        .await
        .expect("analyze should succeed on valid fixture");

    assert!(
        report.total_functions > 0,
        "expected total_functions > 0, got {}",
        report.total_functions
    );
    assert!(
        !report.gaps.is_empty(),
        "expected at least one gap (public_complex is untested)"
    );

    // public_complex should appear among the gaps since it is untested
    let gap_names: Vec<&str> = report
        .gaps
        .iter()
        .map(|g| g.function.name.as_str())
        .collect();
    assert!(
        gap_names.contains(&"public_complex"),
        "expected public_complex in gaps, found: {gap_names:?}"
    );

    assert!(!report.ai_enabled, "AI should be disabled in this test run");
}

#[tokio::test]
async fn test_analyze_empty_dir() {
    let dir = TempDir::new().unwrap();

    let mut config = TestGapConfig::default();
    config.ai.enabled = false;

    let result = analyze(dir.path(), &config).await;

    assert!(
        result.is_err(),
        "analyze on empty dir should return an error"
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err, TestGapError::NoFiles(_)),
        "expected TestGapError::NoFiles, got: {err:?}"
    );
}
