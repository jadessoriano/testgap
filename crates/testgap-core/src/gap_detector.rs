use crate::config::TestGapConfig;
use crate::test_mapper;
use crate::types::{ExtractedFunction, GapSeverity, TestGap};
use std::collections::HashSet;

/// Detect test gaps by comparing source functions against test coverage mapping.
pub fn detect_gaps(
    functions: &[ExtractedFunction],
    covered: &HashSet<String>,
    config: &TestGapConfig,
) -> Vec<TestGap> {
    let mut gaps = Vec::new();

    for func in functions {
        // Skip test functions themselves
        if func.is_test {
            continue;
        }

        let key = test_mapper::function_key(func);
        if covered.contains(&key) {
            continue;
        }

        // Skip trivial functions
        if should_skip(func) {
            continue;
        }

        let severity = classify_severity(func);

        // Apply minimum severity filter
        if severity < config.min_severity {
            continue;
        }

        let reason = build_reason(func, severity);

        gaps.push(TestGap {
            function: func.clone(),
            severity,
            reason,
            ai_analysis: None,
        });
    }

    // Sort by severity (critical first), then by file path
    gaps.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.function.file_path.cmp(&b.function.file_path))
            .then_with(|| a.function.line_start.cmp(&b.function.line_start))
    });

    gaps
}

fn classify_severity(func: &ExtractedFunction) -> GapSeverity {
    let is_complex = func.complexity >= 5;

    match (func.is_public, is_complex) {
        (true, true) => GapSeverity::Critical,
        (true, false) => GapSeverity::Warning,
        (false, _) => GapSeverity::Info,
    }
}

fn build_reason(func: &ExtractedFunction, severity: GapSeverity) -> String {
    match severity {
        GapSeverity::Critical => {
            format!(
                "Public function with high complexity ({}) and no test coverage",
                func.complexity
            )
        }
        GapSeverity::Warning => "Public function with no test coverage".to_string(),
        GapSeverity::Info => "Private function with no test coverage".to_string(),
    }
}

fn should_skip(func: &ExtractedFunction) -> bool {
    let name = &func.name;

    // Skip common boilerplate / trivial functions
    let trivial_names = [
        "main",
        "new",
        "default",
        "fmt",
        "from",
        "into",
        "as_ref",
        "deref",
        "drop",
        "clone",
        "eq",
        "hash",
        "partial_cmp",
        "cmp",
        // Python dunder methods
        "__init__",
        "__str__",
        "__repr__",
        "__eq__",
        "__hash__",
        // Go String() / Error()
        "String",
        "Error",
    ];

    if trivial_names.contains(&name.as_str()) {
        return true;
    }

    // Skip very short functions (getters, simple returns)
    if func.body.lines().count() <= 3 {
        return true;
    }

    false
}
