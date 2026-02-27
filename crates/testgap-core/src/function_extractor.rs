use crate::language_registry;
use crate::test_mapper::SourceFile;
use crate::types::{ExtractedFunction, Language};
use crate::Result;
use crate::TestGapError;
use streaming_iterator::StreamingIterator;

/// Extract all functions from a source file using tree-sitter.
pub fn extract_functions(file: &SourceFile) -> Result<Vec<ExtractedFunction>> {
    let source = std::fs::read_to_string(&file.path).map_err(TestGapError::Io)?;
    let lang = file.language;
    let ts_language = language_registry::get_language(lang);

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&ts_language)
        .map_err(|e| TestGapError::Parse {
            file: file.path.display().to_string(),
            message: e.to_string(),
        })?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| TestGapError::Parse {
            file: file.path.display().to_string(),
            message: "Failed to parse file".into(),
        })?;

    let query_src = language_registry::function_query(lang);
    let query =
        tree_sitter::Query::new(&ts_language, query_src).map_err(|e| TestGapError::Parse {
            file: file.path.display().to_string(),
            message: format!("Query error: {e}"),
        })?;

    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let name_idx = query
        .capture_index_for_name("name")
        .expect("query must have @name capture");
    let func_idx = query
        .capture_index_for_name("function")
        .expect("query must have @function capture");

    let mut functions = Vec::new();

    while let Some(m) = matches.next() {
        let mut name_node = None;
        let mut func_node = None;

        for cap in m.captures {
            if cap.index == name_idx {
                name_node = Some(cap.node);
            } else if cap.index == func_idx {
                func_node = Some(cap.node);
            }
        }

        let (Some(name_n), Some(func_n)) = (name_node, func_node) else {
            continue;
        };

        let name: String = name_n
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            continue;
        }

        let body: String = func_n
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();

        let line_start = func_n.start_position().row + 1;
        let line_end = func_n.end_position().row + 1;

        let signature = extract_signature(&source, func_n, lang);
        let is_public = check_visibility(&source, func_n, lang);
        let is_test = check_is_test(&name, &source, func_n, lang, file.is_test);
        let complexity = estimate_complexity(&body);

        functions.push(ExtractedFunction {
            name,
            file_path: file.path.clone(),
            line_start,
            line_end,
            signature,
            body,
            language: lang,
            is_public,
            is_test,
            complexity,
        });
    }

    // Sort then dedup by (file_path, line_start, name)
    functions.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then_with(|| a.line_start.cmp(&b.line_start))
            .then_with(|| a.name.cmp(&b.name))
    });
    functions.dedup_by(|a, b| {
        a.file_path == b.file_path && a.line_start == b.line_start && a.name == b.name
    });

    Ok(functions)
}

fn extract_signature(source: &str, node: tree_sitter::Node, lang: Language) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    match lang {
        Language::Rust => {
            // Take everything up to the opening brace
            if let Some(brace_pos) = text.find('{') {
                text[..brace_pos].trim().to_string()
            } else {
                text.lines().next().unwrap_or("").to_string()
            }
        }
        Language::Go => {
            if let Some(brace_pos) = text.find('{') {
                text[..brace_pos].trim().to_string()
            } else {
                text.lines().next().unwrap_or("").to_string()
            }
        }
        Language::Python => {
            // Take the def line up to the colon (first line only)
            let first_line = text.lines().next().unwrap_or("");
            if let Some(colon_pos) = first_line.rfind(':') {
                first_line[..colon_pos].trim().to_string()
            } else {
                first_line.to_string()
            }
        }
        Language::JavaScript | Language::TypeScript => {
            // Take the first line or up to opening brace
            if let Some(brace_pos) = text.find('{') {
                text[..brace_pos].trim().to_string()
            } else if let Some(arrow_pos) = text.find("=>") {
                text[..arrow_pos + 2].trim().to_string()
            } else {
                text.lines().next().unwrap_or("").to_string()
            }
        }
    }
}

fn check_visibility(source: &str, node: tree_sitter::Node, lang: Language) -> bool {
    match lang {
        Language::Rust => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            text.starts_with("pub ")
                || text.starts_with("pub(crate)")
                || text.starts_with("pub(super)")
        }
        Language::Python => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            // Python: functions not starting with _ are public
            if let Some(line) = text.lines().next() {
                if let Some(name_start) = line.find("def ") {
                    let after_def = &line[name_start + 4..];
                    return !after_def.starts_with('_');
                }
            }
            true
        }
        Language::Go => {
            // Go: exported functions start with uppercase
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            if let Some(func_pos) = text.find("func ") {
                let after_func = text[func_pos + 5..].trim_start();
                // Skip receiver for methods: (r *Type) Name
                let name_part = if after_func.starts_with('(') {
                    if let Some(paren_end) = after_func.find(") ") {
                        after_func[paren_end + 2..].trim_start()
                    } else {
                        after_func
                    }
                } else {
                    after_func
                };
                name_part.chars().next().is_some_and(|c| c.is_uppercase())
            } else {
                false
            }
        }
        Language::JavaScript | Language::TypeScript => {
            // Check if parent is an export statement
            if let Some(parent) = node.parent() {
                let kind = parent.kind();
                kind == "export_statement" || kind == "export_default_declaration"
            } else {
                // Top-level functions without export — treat as module-public
                true
            }
        }
    }
}

fn check_is_test(
    name: &str,
    source: &str,
    node: tree_sitter::Node,
    lang: Language,
    is_test_file: bool,
) -> bool {
    if is_test_file {
        return true;
    }

    match lang {
        Language::Rust => {
            // Check for #[test] or #[cfg(test)] attribute (walk all preceding siblings)
            let mut sibling = node.prev_sibling();
            while let Some(prev) = sibling {
                if prev.kind() == "attribute_item" {
                    let attr_text = prev.utf8_text(source.as_bytes()).unwrap_or("");
                    if attr_text.contains("test") {
                        return true;
                    }
                } else {
                    break;
                }
                sibling = prev.prev_sibling();
            }
            name.starts_with("test_")
        }
        Language::Python => name.starts_with("test_"),
        Language::Go => name.starts_with("Test") || name.starts_with("Benchmark"),
        Language::JavaScript | Language::TypeScript => {
            name == "it" || name == "test" || name == "describe"
        }
    }
}

fn estimate_complexity(body: &str) -> u32 {
    // Simple cyclomatic complexity estimate: count branching keywords
    let keywords = [
        "if ", "else ", "else{", "match ", "for ", "while ", "loop ", "case ", "catch ", "except ",
        "elif ", "?", "&&", "||", "switch ",
    ];
    let mut complexity: u32 = 1; // base complexity
    for kw in &keywords {
        complexity += body.matches(kw).count() as u32;
    }
    complexity
}

#[cfg(test)]
mod tests {
    use crate::function_extractor::extract_functions;
    use crate::test_mapper::SourceFile;
    use crate::types::Language;
    use std::io::Write;
    use tempfile;

    #[test]
    fn parse_rust_snippet() {
        let mut file = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        writeln!(
            file,
            r#"pub fn add(a: i32, b: i32) -> i32 {{
    a + b
}}

pub fn complex_calc(x: i32) -> i32 {{
    if x > 0 {{
        for i in 0..x {{
            if i % 2 == 0 {{
                return i;
            }}
        }}
    }}
    x
}}"#
        )
        .unwrap();
        file.flush().unwrap();

        let source = SourceFile {
            path: file.path().to_path_buf(),
            language: Language::Rust,
            is_test: false,
        };

        let funcs = extract_functions(&source).unwrap();
        assert!(
            funcs.len() >= 2,
            "expected at least 2 functions, got {}",
            funcs.len()
        );

        let add_fn = funcs
            .iter()
            .find(|f| f.name == "add")
            .expect("should find 'add'");
        assert!(add_fn.is_public, "add should be public");
        assert!(add_fn.line_start >= 1);
        assert!(add_fn.line_end >= add_fn.line_start);
        assert!(
            add_fn.signature.contains("fn add"),
            "signature should contain 'fn add', got: {}",
            add_fn.signature
        );

        let complex_fn = funcs
            .iter()
            .find(|f| f.name == "complex_calc")
            .expect("should find 'complex_calc'");
        assert!(complex_fn.is_public, "complex_calc should be public");
    }

    #[test]
    fn parse_typescript_snippet() {
        let mut file = tempfile::Builder::new().suffix(".ts").tempfile().unwrap();
        writeln!(
            file,
            r#"export function greet(name: string): string {{
    return "hello " + name;
}}"#
        )
        .unwrap();
        file.flush().unwrap();

        let source = SourceFile {
            path: file.path().to_path_buf(),
            language: Language::TypeScript,
            is_test: false,
        };

        let funcs = extract_functions(&source).unwrap();
        assert!(!funcs.is_empty(), "expected at least 1 function");

        let greet_fn = funcs
            .iter()
            .find(|f| f.name == "greet")
            .expect("should find 'greet'");
        assert!(greet_fn.is_public, "exported function should be public");
        assert_eq!(greet_fn.name, "greet");
    }

    #[test]
    fn parse_python_snippet() {
        let mut file = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
        write!(
            file,
            "def calculate(x, y):\n    if x > 0:\n        return x + y\n    return y\n"
        )
        .unwrap();
        file.flush().unwrap();

        let source = SourceFile {
            path: file.path().to_path_buf(),
            language: Language::Python,
            is_test: false,
        };

        let funcs = extract_functions(&source).unwrap();
        assert!(!funcs.is_empty(), "expected at least 1 function");

        let calc_fn = funcs
            .iter()
            .find(|f| f.name == "calculate")
            .expect("should find 'calculate'");
        assert!(
            calc_fn.is_public,
            "calculate should be public (no leading underscore)"
        );
        assert_eq!(calc_fn.name, "calculate");
    }

    #[test]
    fn complexity_estimation_via_extract() {
        let mut file = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        writeln!(
            file,
            r#"pub fn branchy(x: i32, y: i32) -> i32 {{
    if x > 0 {{
        if y > 0 {{
            for i in 0..x {{
                match i {{
                    0 => return 0,
                    _ => {{
                        if i > 5 && y < 10 {{
                            return i;
                        }}
                    }}
                }}
            }}
        }}
    }} else {{
        while x > 0 {{
            return y;
        }}
    }}
    x + y
}}"#
        )
        .unwrap();
        file.flush().unwrap();

        let source = SourceFile {
            path: file.path().to_path_buf(),
            language: Language::Rust,
            is_test: false,
        };

        let funcs = extract_functions(&source).unwrap();
        let branchy = funcs
            .iter()
            .find(|f| f.name == "branchy")
            .expect("should find 'branchy'");
        assert!(
            branchy.complexity > 1,
            "complex function should have complexity > 1, got {}",
            branchy.complexity
        );
    }
}
