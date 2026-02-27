use crate::config::TestGapConfig;
use crate::language_registry;
use crate::types::{ExtractedFunction, Language};
use crate::Result;
use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub language: Language,
    pub is_test: bool,
}

#[derive(Debug)]
pub struct ScannedFiles {
    pub source_files: Vec<SourceFile>,
    pub test_files: Vec<SourceFile>,
}

/// Scan a directory for source and test files, respecting config excludes.
pub fn scan_directory(root: &Path, config: &TestGapConfig) -> Result<ScannedFiles> {
    let exclude_patterns: Vec<glob::Pattern> = config
        .exclude
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    let allowed_languages: Option<HashSet<Language>> = config
        .languages
        .as_ref()
        .map(|v| v.iter().copied().collect());

    let mut source_files = Vec::new();
    let mut test_files = Vec::new();

    for entry in WalkBuilder::new(root)
        .follow_links(false)
        .hidden(true)
        .build()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        let relative_str = relative.to_string_lossy();

        // Check excludes
        if exclude_patterns.iter().any(|p| p.matches(&relative_str)) {
            continue;
        }

        // Detect language
        let Some(lang) = language_registry::detect_language(path) else {
            continue;
        };

        // Filter by allowed languages
        if let Some(ref allowed) = allowed_languages {
            if !allowed.contains(&lang) {
                continue;
            }
        }

        let is_test = language_registry::is_test_file(
            relative,
            &config.test_patterns.test_dirs,
            &config.test_patterns.test_file_suffixes,
            &config.test_patterns.test_file_prefixes,
        );

        let file = SourceFile {
            path: path.to_path_buf(),
            language: lang,
            is_test,
        };

        if is_test {
            test_files.push(file);
        } else {
            source_files.push(file);
        }
    }

    tracing::info!(
        "Found {} source files and {} test files",
        source_files.len(),
        test_files.len()
    );

    Ok(ScannedFiles {
        source_files,
        test_files,
    })
}

/// Map test functions to source functions they likely cover.
/// Returns a set of source function identifiers that have test coverage.
pub fn map_tests_to_functions(
    source_functions: &[ExtractedFunction],
    test_functions: &[ExtractedFunction],
) -> HashSet<String> {
    let mut covered = HashSet::new();

    // Build a lookup by function name
    let source_by_name: HashMap<&str, Vec<&ExtractedFunction>> = {
        let mut map = HashMap::new();
        for f in source_functions {
            if !f.is_test {
                map.entry(f.name.as_str()).or_insert_with(Vec::new).push(f);
            }
        }
        map
    };

    for test_fn in test_functions {
        let test_name = &test_fn.name;

        // Strategy 1: Direct name matching
        // test_foo → foo, test_Foo → Foo
        if let Some(target) = test_name.strip_prefix("test_") {
            if source_by_name.contains_key(target) {
                covered.insert(make_key(target, source_by_name[target][0]));
            }
        }

        // Strategy 2: Go-style Test prefix
        if let Some(target) = test_name.strip_prefix("Test") {
            let lower = to_snake_case(target);
            if source_by_name.contains_key(lower.as_str()) {
                covered.insert(make_key(&lower, source_by_name[lower.as_str()][0]));
            }
            // Also try the PascalCase name directly
            if source_by_name.contains_key(target) {
                covered.insert(make_key(target, source_by_name[target][0]));
            }
        }

        // Strategy 3: Body references — scan test body for source function names
        let body_lower = test_fn.body.to_lowercase();
        for (name, funcs) in &source_by_name {
            if body_lower.contains(&name.to_lowercase()) {
                covered.insert(make_key(name, funcs[0]));
            }
        }

        // Strategy 4: File-based mapping — test_foo.rs tests foo.rs
        if let Some(test_stem) = test_fn.file_path.file_stem().and_then(|s| s.to_str()) {
            let candidate = test_stem
                .strip_prefix("test_")
                .or_else(|| test_stem.strip_suffix("_test"))
                .or_else(|| test_stem.strip_suffix(".test"))
                .or_else(|| test_stem.strip_suffix(".spec"))
                .or_else(|| test_stem.strip_suffix("_spec"));

            if let Some(source_stem) = candidate {
                for (name, funcs) in &source_by_name {
                    if funcs.iter().any(|f| {
                        f.file_path.file_stem().and_then(|s| s.to_str()) == Some(source_stem)
                    }) {
                        covered.insert(make_key(name, funcs[0]));
                    }
                }
            }
        }
    }

    covered
}

fn make_key(name: &str, func: &ExtractedFunction) -> String {
    format!("{}::{}", func.file_path.display(), name)
}

pub fn function_key(func: &ExtractedFunction) -> String {
    make_key(&func.name, func)
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::test_mapper::{function_key, map_tests_to_functions};
    use crate::types::*;
    use std::path::PathBuf;

    fn make_func(name: &str, path: &str, is_test: bool, body: &str) -> ExtractedFunction {
        ExtractedFunction {
            name: name.to_string(),
            file_path: PathBuf::from(path),
            line_start: 1,
            line_end: 10,
            signature: format!("fn {name}()"),
            body: body.to_string(),
            language: Language::Rust,
            is_public: true,
            is_test,
            complexity: 1,
        }
    }

    #[test]
    fn name_matching_test_prefix() {
        let source_funcs = vec![make_func("foo", "src/lib.rs", false, "fn foo() {}")];
        let test_funcs = vec![make_func(
            "test_foo",
            "tests/lib_test.rs",
            true,
            "fn test_foo() { foo(); }",
        )];

        let covered = map_tests_to_functions(&source_funcs, &test_funcs);
        let key = function_key(&source_funcs[0]);
        assert!(
            covered.contains(&key),
            "test_foo should cover foo, covered set: {:?}",
            covered
        );
    }

    #[test]
    fn go_style_test_prefix() {
        let source_funcs = vec![make_func(
            "calculate",
            "src/math.rs",
            false,
            "fn calculate() {}",
        )];
        let test_funcs = vec![make_func(
            "TestCalculate",
            "tests/math_test.rs",
            true,
            "fn TestCalculate() {}",
        )];

        let covered = map_tests_to_functions(&source_funcs, &test_funcs);
        let key = function_key(&source_funcs[0]);
        assert!(
            covered.contains(&key),
            "TestCalculate should cover calculate via snake_case conversion, covered set: {:?}",
            covered
        );
    }

    #[test]
    fn body_matching() {
        let source_funcs = vec![make_func(
            "process_data",
            "src/processor.rs",
            false,
            "fn process_data() { /* impl */ }",
        )];
        let test_funcs = vec![make_func(
            "test_integration",
            "tests/integration.rs",
            true,
            "fn test_integration() { process_data(input); assert!(result); }",
        )];

        let covered = map_tests_to_functions(&source_funcs, &test_funcs);
        let key = function_key(&source_funcs[0]);
        assert!(
            covered.contains(&key),
            "test body containing 'process_data' should cover it, covered set: {:?}",
            covered
        );
    }

    #[test]
    fn function_key_format() {
        let func = make_func("my_func", "src/lib.rs", false, "fn my_func() {}");
        let key = function_key(&func);
        assert_eq!(key, "src/lib.rs::my_func");
    }

    #[test]
    fn uncovered_function_not_in_set() {
        let source_funcs = vec![
            make_func("covered_fn", "src/lib.rs", false, "fn covered_fn() {}"),
            make_func("uncovered_fn", "src/lib.rs", false, "fn uncovered_fn() {}"),
        ];
        let test_funcs = vec![make_func(
            "test_covered_fn",
            "tests/test.rs",
            true,
            "fn test_covered_fn() {}",
        )];

        let covered = map_tests_to_functions(&source_funcs, &test_funcs);
        let covered_key = function_key(&source_funcs[0]);
        let uncovered_key = function_key(&source_funcs[1]);

        assert!(
            covered.contains(&covered_key),
            "covered_fn should be covered"
        );
        assert!(
            !covered.contains(&uncovered_key),
            "uncovered_fn should NOT be covered"
        );
    }
}
