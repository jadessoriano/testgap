use crate::types::{GapSeverity, Language};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = ".testgap.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGapConfig {
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,

    #[serde(default)]
    pub languages: Option<Vec<Language>>,

    #[serde(default = "default_min_severity")]
    pub min_severity: GapSeverity,

    #[serde(default = "default_format")]
    pub format: OutputFormat,

    #[serde(default)]
    pub ai: AiConfig,

    #[serde(default)]
    pub test_patterns: TestPatternConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Human,
    Json,
    Markdown,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Human => f.write_str("human"),
            OutputFormat::Json => f.write_str("json"),
            OutputFormat::Markdown => f.write_str("markdown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    #[serde(default = "default_max_function_body_tokens")]
    pub max_function_body_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPatternConfig {
    #[serde(default = "default_test_dirs")]
    pub test_dirs: Vec<String>,

    #[serde(default = "default_test_suffixes")]
    pub test_file_suffixes: Vec<String>,

    #[serde(default = "default_test_prefixes")]
    pub test_file_prefixes: Vec<String>,
}

fn default_exclude() -> Vec<String> {
    vec![
        "**/target/**".into(),
        "**/node_modules/**".into(),
        "**/.git/**".into(),
        "**/dist/**".into(),
        "**/build/**".into(),
        "**/vendor/**".into(),
        "**/__pycache__/**".into(),
        "**/.venv/**".into(),
    ]
}

fn default_min_severity() -> GapSeverity {
    GapSeverity::Info
}

fn default_format() -> OutputFormat {
    OutputFormat::Human
}

fn default_true() -> bool {
    true
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".into()
}

fn default_batch_size() -> usize {
    5
}

fn default_max_function_body_tokens() -> usize {
    2000
}

fn default_test_dirs() -> Vec<String> {
    vec![
        "tests".into(),
        "test".into(),
        "__tests__".into(),
        "spec".into(),
    ]
}

fn default_test_suffixes() -> Vec<String> {
    vec![
        "_test".into(),
        ".test".into(),
        ".spec".into(),
        "_spec".into(),
    ]
}

fn default_test_prefixes() -> Vec<String> {
    vec!["test_".into()]
}

impl Default for TestGapConfig {
    fn default() -> Self {
        Self {
            exclude: default_exclude(),
            languages: None,
            min_severity: default_min_severity(),
            format: default_format(),
            ai: AiConfig::default(),
            test_patterns: TestPatternConfig::default(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: default_model(),
            batch_size: default_batch_size(),
            max_function_body_tokens: default_max_function_body_tokens(),
        }
    }
}

impl Default for TestPatternConfig {
    fn default() -> Self {
        Self {
            test_dirs: default_test_dirs(),
            test_file_suffixes: default_test_suffixes(),
            test_file_prefixes: default_test_prefixes(),
        }
    }
}

impl TestGapConfig {
    /// Search upward from `start` for `.testgap.toml`, falling back to defaults.
    pub fn load(start: &Path) -> Self {
        if let Some(path) = find_config_file(start) {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => {
                        tracing::info!("Loaded config from {}", path.display());
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {e}", path.display());
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {e}", path.display());
                }
            }
        }
        tracing::info!("No .testgap.toml found, using defaults");
        Self::default()
    }

    /// Merge CLI overrides into this config.
    pub fn merge_cli_overrides(
        &mut self,
        format: Option<OutputFormat>,
        languages: Option<Vec<Language>>,
        min_severity: Option<GapSeverity>,
        no_ai: bool,
    ) {
        if let Some(f) = format {
            self.format = f;
        }
        if let Some(l) = languages {
            self.languages = Some(l);
        }
        if let Some(s) = min_severity {
            self.min_severity = s;
        }
        if no_ai {
            self.ai.enabled = false;
        }
    }
}

fn find_config_file(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = dir.join(CONFIG_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn generate_default_config() -> String {
    r#"# testgap configuration
# Place this file at the root of your project as .testgap.toml

# Glob patterns to exclude from analysis
exclude = [
    "**/target/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/dist/**",
    "**/build/**",
    "**/vendor/**",
    "**/__pycache__/**",
    "**/.venv/**",
]

# Restrict analysis to specific languages (comment out to auto-detect)
# languages = ["rust", "typescript", "python"]

# Minimum severity to report: "info", "warning", or "critical"
min_severity = "info"

# Output format: "human", "json", or "markdown"
format = "human"

[ai]
enabled = true
model = "claude-sonnet-4-20250514"
batch_size = 5
max_function_body_tokens = 2000

[test_patterns]
test_dirs = ["tests", "test", "__tests__", "spec"]
test_file_suffixes = ["_test", ".test", ".spec", "_spec"]
test_file_prefixes = ["test_"]
"#
    .to_string()
}
