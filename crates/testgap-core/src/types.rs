use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Python => "python",
            Language::Go => "go",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFunction {
    pub name: String,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
    pub body: String,
    pub language: Language,
    pub is_public: bool,
    pub is_test: bool,
    pub complexity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GapSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for GapSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GapSeverity::Info => f.write_str("info"),
            GapSeverity::Warning => f.write_str("warning"),
            GapSeverity::Critical => f.write_str("critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGap {
    pub function: ExtractedFunction,
    pub severity: GapSeverity,
    pub reason: String,
    pub ai_analysis: Option<AiAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysis {
    pub risk_assessment: String,
    pub suggested_tests: Vec<String>,
    pub priority_score: u8,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub project_path: PathBuf,
    pub total_functions: usize,
    pub tested_functions: usize,
    pub gaps: Vec<TestGap>,
    pub languages_analyzed: Vec<Language>,
    pub ai_enabled: bool,
    pub token_usage: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl AnalysisReport {
    pub fn coverage_percent(&self) -> f64 {
        if self.total_functions == 0 {
            return 100.0;
        }
        (self.tested_functions as f64 / self.total_functions as f64) * 100.0
    }

    pub fn has_critical_gaps(&self) -> bool {
        self.gaps
            .iter()
            .any(|g| g.severity == GapSeverity::Critical)
    }

    pub fn gaps_by_severity(&self, severity: GapSeverity) -> Vec<&TestGap> {
        self.gaps
            .iter()
            .filter(|g| g.severity == severity)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────

    fn make_function(name: &str) -> ExtractedFunction {
        ExtractedFunction {
            name: name.into(),
            file_path: PathBuf::from("src/lib.rs"),
            line_start: 1,
            line_end: 10,
            signature: format!("fn {name}()"),
            body: "{}".into(),
            language: Language::Rust,
            is_public: true,
            is_test: false,
            complexity: 1,
        }
    }

    fn make_gap(name: &str, severity: GapSeverity) -> TestGap {
        TestGap {
            function: make_function(name),
            severity,
            reason: "untested".into(),
            ai_analysis: None,
        }
    }

    fn make_report(total: usize, tested: usize, gaps: Vec<TestGap>) -> AnalysisReport {
        AnalysisReport {
            project_path: PathBuf::from("/tmp/project"),
            total_functions: total,
            tested_functions: tested,
            gaps,
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        }
    }

    // ── Language::from_extension ─────────────────────────────────────

    #[test]
    fn from_extension_rust() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    }

    #[test]
    fn from_extension_javascript_variants() {
        for ext in &["js", "jsx", "mjs", "cjs"] {
            assert_eq!(
                Language::from_extension(ext),
                Some(Language::JavaScript),
                "failed for extension {ext}"
            );
        }
    }

    #[test]
    fn from_extension_typescript_variants() {
        for ext in &["ts", "tsx", "mts", "cts"] {
            assert_eq!(
                Language::from_extension(ext),
                Some(Language::TypeScript),
                "failed for extension {ext}"
            );
        }
    }

    #[test]
    fn from_extension_python() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
    }

    #[test]
    fn from_extension_go() {
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
    }

    #[test]
    fn from_extension_unknown_returns_none() {
        for ext in &["txt", "md", ""] {
            assert_eq!(
                Language::from_extension(ext),
                None,
                "expected None for extension {ext}"
            );
        }
    }

    // ── coverage_percent ────────────────────────────────────────────

    #[test]
    fn coverage_percent_zero_functions_returns_100() {
        let report = make_report(0, 0, vec![]);
        assert!((report.coverage_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn coverage_percent_partial() {
        let report = make_report(10, 3, vec![]);
        assert!((report.coverage_percent() - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn coverage_percent_all_tested() {
        let report = make_report(10, 10, vec![]);
        assert!((report.coverage_percent() - 100.0).abs() < f64::EPSILON);
    }

    // ── has_critical_gaps ───────────────────────────────────────────

    #[test]
    fn has_critical_gaps_true_when_critical_present() {
        let gaps = vec![
            make_gap("a", GapSeverity::Info),
            make_gap("b", GapSeverity::Critical),
        ];
        let report = make_report(5, 3, gaps);
        assert!(report.has_critical_gaps());
    }

    #[test]
    fn has_critical_gaps_false_when_no_critical() {
        let gaps = vec![
            make_gap("a", GapSeverity::Info),
            make_gap("b", GapSeverity::Warning),
        ];
        let report = make_report(5, 3, gaps);
        assert!(!report.has_critical_gaps());
    }

    #[test]
    fn has_critical_gaps_false_when_empty() {
        let report = make_report(5, 5, vec![]);
        assert!(!report.has_critical_gaps());
    }

    // ── gaps_by_severity ────────────────────────────────────────────

    #[test]
    fn gaps_by_severity_filters_correctly() {
        let gaps = vec![
            make_gap("a", GapSeverity::Info),
            make_gap("b", GapSeverity::Warning),
            make_gap("c", GapSeverity::Critical),
            make_gap("d", GapSeverity::Warning),
        ];
        let report = make_report(10, 6, gaps);

        assert_eq!(report.gaps_by_severity(GapSeverity::Info).len(), 1);
        assert_eq!(report.gaps_by_severity(GapSeverity::Warning).len(), 2);
        assert_eq!(report.gaps_by_severity(GapSeverity::Critical).len(), 1);
    }

    // ── GapSeverity ordering ────────────────────────────────────────

    #[test]
    fn gap_severity_ordering() {
        assert!(GapSeverity::Info < GapSeverity::Warning);
        assert!(GapSeverity::Warning < GapSeverity::Critical);
        assert!(GapSeverity::Info < GapSeverity::Critical);
    }

    #[test]
    fn gap_severity_sorted_vec() {
        let mut severities = vec![
            GapSeverity::Critical,
            GapSeverity::Info,
            GapSeverity::Warning,
        ];
        severities.sort();
        assert_eq!(
            severities,
            vec![
                GapSeverity::Info,
                GapSeverity::Warning,
                GapSeverity::Critical
            ]
        );
    }
}
