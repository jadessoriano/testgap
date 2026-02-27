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

/// Returns the tree-sitter query for extracting test functions.
pub fn test_query(lang: Language) -> &'static str {
    match lang {
        Language::Rust => {
            r#"
            (attribute_item
                (attribute
                    (identifier) @attr_name
                )
            )

            (function_item
                name: (identifier) @name
            ) @function
            "#
        }
        Language::JavaScript | Language::TypeScript => {
            // Match describe/it/test call expressions
            r#"
            (call_expression
                function: (identifier) @call_name
                arguments: (arguments
                    (string) @test_name
                )
            ) @function
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
                parameters: (parameter_list
                    (parameter_declaration
                        type: (pointer_type
                            (qualified_type) @param_type
                        )
                    )
                )
            ) @function
            "#
        }
    }
}

/// Check if a file path is a test file based on naming conventions.
pub fn is_test_file(path: &Path, test_dirs: &[String], suffixes: &[String], prefixes: &[String]) -> bool {
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
