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
        self.gaps.iter().any(|g| g.severity == GapSeverity::Critical)
    }

    pub fn gaps_by_severity(&self, severity: GapSeverity) -> Vec<&TestGap> {
        self.gaps.iter().filter(|g| g.severity == severity).collect()
    }
}
