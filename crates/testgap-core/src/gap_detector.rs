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

#[cfg(test)]
mod tests {
    use crate::config::TestGapConfig;
    use crate::gap_detector::detect_gaps;
    use crate::test_mapper;
    use crate::types::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn make_func(
        name: &str,
        is_public: bool,
        complexity: u32,
        body_lines: usize,
    ) -> ExtractedFunction {
        let body = (0..body_lines)
            .map(|i| format!("    line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        ExtractedFunction {
            name: name.to_string(),
            file_path: PathBuf::from("src/lib.rs"),
            line_start: 1,
            line_end: body_lines,
            signature: format!("fn {name}()"),
            body,
            language: Language::Rust,
            is_public,
            is_test: false,
            complexity,
        }
    }

    #[test]
    fn severity_critical_for_public_complex() {
        let func = make_func("process_data", true, 5, 10);
        let covered = HashSet::new();
        let config = TestGapConfig::default();

        let gaps = detect_gaps(&[func], &covered, &config);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Critical);
    }

    #[test]
    fn severity_warning_for_public_simple() {
        let func = make_func("get_value", true, 2, 10);
        let covered = HashSet::new();
        let config = TestGapConfig::default();

        let gaps = detect_gaps(&[func], &covered, &config);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Warning);
    }

    #[test]
    fn severity_info_for_private() {
        let func = make_func("helper_internal", false, 10, 10);
        let covered = HashSet::new();
        let config = TestGapConfig::default();

        let gaps = detect_gaps(&[func], &covered, &config);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Info);
    }

    #[test]
    fn should_skip_trivial_names() {
        let trivial_names = [
            "main", "new", "default", "fmt", "from", "__init__", "String", "Error",
        ];
        let config = TestGapConfig::default();
        let covered = HashSet::new();

        for name in &trivial_names {
            let func = make_func(name, true, 3, 10);
            let gaps = detect_gaps(&[func], &covered, &config);
            assert!(gaps.is_empty(), "function '{}' should be skipped", name);
        }
    }

    #[test]
    fn should_skip_short_body() {
        // Functions with body <= 3 lines should be skipped
        let func = make_func("short_func", true, 3, 3);
        let config = TestGapConfig::default();
        let covered = HashSet::new();

        let gaps = detect_gaps(&[func], &covered, &config);
        assert!(
            gaps.is_empty(),
            "function with body <= 3 lines should be skipped"
        );
    }

    #[test]
    fn detect_gaps_end_to_end() {
        let funcs = vec![
            make_func("uncovered_public_complex", true, 7, 15),
            make_func("uncovered_public_simple", true, 2, 8),
            make_func("uncovered_private", false, 3, 6),
            make_func("covered_func", true, 3, 10),
        ];

        let covered_key = test_mapper::function_key(&funcs[3]);
        let mut covered = HashSet::new();
        covered.insert(covered_key);

        let config = TestGapConfig::default();
        let gaps = detect_gaps(&funcs, &covered, &config);

        // Should have 3 gaps (the uncovered ones)
        assert_eq!(gaps.len(), 3, "expected 3 gaps, got {}", gaps.len());

        // First gap should be Critical (sorted by severity descending)
        assert_eq!(gaps[0].severity, GapSeverity::Critical);
        assert_eq!(gaps[0].function.name, "uncovered_public_complex");

        // Second should be Warning
        assert_eq!(gaps[1].severity, GapSeverity::Warning);
        assert_eq!(gaps[1].function.name, "uncovered_public_simple");

        // Third should be Info
        assert_eq!(gaps[2].severity, GapSeverity::Info);
        assert_eq!(gaps[2].function.name, "uncovered_private");
    }

    #[test]
    fn test_functions_are_excluded() {
        let mut func = make_func("test_something", true, 3, 10);
        func.is_test = true;
        let config = TestGapConfig::default();
        let covered = HashSet::new();

        let gaps = detect_gaps(&[func], &covered, &config);
        assert!(gaps.is_empty(), "test functions should not appear as gaps");
    }
}
