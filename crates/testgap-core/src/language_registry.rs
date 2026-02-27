use crate::types::Language;
use std::path::Path;

/// Returns the tree-sitter language for parsing.
pub fn get_language(lang: Language) -> tree_sitter::Language {
    match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

/// Returns the tree-sitter query for extracting function definitions.
pub fn function_query(lang: Language) -> &'static str {
    match lang {
        Language::Rust => {
            r#"
            (function_item
                name: (identifier) @name
            ) @function
            "#
        }
        Language::JavaScript => {
            r#"
            (function_declaration
                name: (identifier) @name
            ) @function

            (export_statement
                declaration: (function_declaration
                    name: (identifier) @name
                ) @function
            )

            (lexical_declaration
                (variable_declarator
                    name: (identifier) @name
                    value: (arrow_function) @function
                )
            )

            (variable_declaration
                (variable_declarator
                    name: (identifier) @name
                    value: (arrow_function) @function
                )
            )
            "#
        }
        Language::TypeScript => {
            // TypeScript uses the same patterns as JavaScript plus type annotations
            r#"
            (function_declaration
                name: (identifier) @name
            ) @function

            (export_statement
                declaration: (function_declaration
                    name: (identifier) @name
                ) @function
            )

            (lexical_declaration
                (variable_declarator
                    name: (identifier) @name
                    value: (arrow_function) @function
                )
            )
            "#
        }
        Language::Python => {
            r#"
            (function_definition
                name: (identifier) @name
            ) @function
            "#
        }
        Language::Go => {
            r#"
            (function_declaration
                name: (identifier) @name
            ) @function

            (method_declaration
                name: (field_identifier) @name
            ) @function
            "#
        }
    }
}

/// Check if a file path is a test file based on naming conventions.
pub fn is_test_file(
    path: &Path,
    test_dirs: &[String],
    suffixes: &[String],
    prefixes: &[String],
) -> bool {
    let path_str = path.to_string_lossy();

    // Check if the file is under a test directory
    for dir in test_dirs {
        if path_str.contains(&format!("/{dir}/")) || path_str.contains(&format!("\\{dir}\\")) {
            return true;
        }
    }

    // Check file name patterns
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        // Handle double extensions like .test.ts, .spec.ts
        let full_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        for suffix in suffixes {
            if stem.ends_with(suffix) || full_name.contains(&format!("{suffix}.")) {
                return true;
            }
        }
        for prefix in prefixes {
            if stem.starts_with(prefix) {
                return true;
            }
        }
    }

    // Rust-specific: check for inline #[cfg(test)] modules (handled at parse time)
    false
}

/// Detect the language from a file path based on its extension.
pub fn detect_language(path: &Path) -> Option<Language> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(Language::from_extension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Default test patterns (matching config defaults)
    fn test_dirs() -> Vec<String> {
        vec![
            "tests".into(),
            "test".into(),
            "__tests__".into(),
            "spec".into(),
        ]
    }

    fn test_suffixes() -> Vec<String> {
        vec![
            "_test".into(),
            ".test".into(),
            ".spec".into(),
            "_spec".into(),
        ]
    }

    fn test_prefixes() -> Vec<String> {
        vec!["test_".into()]
    }

    // ── detect_language ─────────────────────────────────────────────

    #[test]
    fn detect_language_rust() {
        assert_eq!(
            detect_language(Path::new("src/lib.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn detect_language_javascript() {
        assert_eq!(
            detect_language(Path::new("index.js")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn detect_language_jsx() {
        assert_eq!(
            detect_language(Path::new("App.jsx")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn detect_language_typescript() {
        assert_eq!(
            detect_language(Path::new("main.ts")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detect_language_tsx() {
        assert_eq!(
            detect_language(Path::new("App.tsx")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detect_language_python() {
        assert_eq!(
            detect_language(Path::new("main.py")),
            Some(Language::Python)
        );
    }

    #[test]
    fn detect_language_go() {
        assert_eq!(detect_language(Path::new("main.go")), Some(Language::Go));
    }

    #[test]
    fn detect_language_unsupported_returns_none() {
        assert_eq!(detect_language(Path::new("README.txt")), None);
        assert_eq!(detect_language(Path::new("docs.md")), None);
    }

    #[test]
    fn detect_language_no_extension_returns_none() {
        assert_eq!(detect_language(Path::new("Makefile")), None);
    }

    // ── is_test_file: true cases ────────────────────────────────────

    #[test]
    fn is_test_file_in_tests_dir() {
        let path = PathBuf::from("project/tests/foo.rs");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_in_test_dir() {
        let path = PathBuf::from("project/test/helper.js");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_in_dunder_tests_dir() {
        let path = PathBuf::from("src/__tests__/App.test.tsx");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_in_spec_dir() {
        let path = PathBuf::from("project/spec/helper_spec.rb");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_with_test_suffix() {
        let path = PathBuf::from("src/foo_test.rs");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_with_dot_test_suffix() {
        let path = PathBuf::from("src/foo.test.ts");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_with_spec_suffix() {
        let path = PathBuf::from("src/foo.spec.js");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_with_test_prefix() {
        let path = PathBuf::from("src/test_foo.py");
        assert!(is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    // ── is_test_file: false cases ───────────────────────────────────

    #[test]
    fn is_test_file_regular_source_rs() {
        let path = PathBuf::from("src/lib.rs");
        assert!(!is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_regular_source_go() {
        let path = PathBuf::from("cmd/main.go");
        assert!(!is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }

    #[test]
    fn is_test_file_regular_source_py() {
        let path = PathBuf::from("src/utils.py");
        assert!(!is_test_file(
            &path,
            &test_dirs(),
            &test_suffixes(),
            &test_prefixes()
        ));
    }
}
