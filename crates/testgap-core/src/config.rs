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

#[cfg(test)]
mod tests {
    use super::*;

    // ── default config values ───────────────────────────────────────

    #[test]
    fn default_config_exclude_patterns() {
        let cfg = TestGapConfig::default();
        let patterns: Vec<&str> = cfg.exclude.iter().map(|s| s.as_str()).collect();
        assert!(patterns.contains(&"**/target/**"), "should contain target");
        assert!(
            patterns.contains(&"**/node_modules/**"),
            "should contain node_modules"
        );
    }

    #[test]
    fn default_config_format_is_human() {
        let cfg = TestGapConfig::default();
        assert_eq!(cfg.format, OutputFormat::Human);
    }

    #[test]
    fn default_config_ai_enabled() {
        let cfg = TestGapConfig::default();
        assert!(cfg.ai.enabled);
    }

    #[test]
    fn default_config_ai_model() {
        let cfg = TestGapConfig::default();
        assert_eq!(cfg.ai.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn default_config_ai_batch_size() {
        let cfg = TestGapConfig::default();
        assert_eq!(cfg.ai.batch_size, 5);
    }

    // ── load from valid TOML file ───────────────────────────────────

    #[test]
    fn load_from_valid_toml_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILENAME);
        std::fs::write(&config_path, r#"min_severity = "warning""#).unwrap();

        let cfg = TestGapConfig::load(dir.path());
        assert_eq!(cfg.min_severity, GapSeverity::Warning);
    }

    // ── load from non-existent dir falls back to defaults ───────────

    #[test]
    fn load_from_nonexistent_dir_uses_defaults() {
        let cfg = TestGapConfig::load(Path::new("/tmp/nonexistent_testgap_dir_12345"));
        // Should be the same as defaults
        assert_eq!(cfg.format, OutputFormat::Human);
        assert!(cfg.ai.enabled);
        assert_eq!(cfg.min_severity, GapSeverity::Info);
    }

    // ── merge_cli_overrides ─────────────────────────────────────────

    #[test]
    fn merge_cli_overrides_sets_format() {
        let mut cfg = TestGapConfig::default();
        cfg.merge_cli_overrides(Some(OutputFormat::Json), None, None, false);
        assert_eq!(cfg.format, OutputFormat::Json);
    }

    #[test]
    fn merge_cli_overrides_no_ai_disables_ai() {
        let mut cfg = TestGapConfig::default();
        assert!(cfg.ai.enabled);
        cfg.merge_cli_overrides(None, None, None, true);
        assert!(!cfg.ai.enabled);
    }

    #[test]
    fn merge_cli_overrides_sets_languages() {
        let mut cfg = TestGapConfig::default();
        cfg.merge_cli_overrides(
            None,
            Some(vec![Language::Rust, Language::Python]),
            None,
            false,
        );
        assert_eq!(cfg.languages, Some(vec![Language::Rust, Language::Python]));
    }

    #[test]
    fn merge_cli_overrides_sets_min_severity() {
        let mut cfg = TestGapConfig::default();
        cfg.merge_cli_overrides(None, None, Some(GapSeverity::Critical), false);
        assert_eq!(cfg.min_severity, GapSeverity::Critical);
    }

    // ── walk-up config file discovery ───────────────────────────────

    #[test]
    fn load_walks_up_to_find_config() {
        let root = tempfile::tempdir().unwrap();
        let a = root.path().join("a");
        let b = a.join("b");
        let c = b.join("c");
        std::fs::create_dir_all(&c).unwrap();

        // Place config in "a/"
        let config_path = a.join(CONFIG_FILENAME);
        std::fs::write(&config_path, r#"min_severity = "critical""#).unwrap();

        // Load from "a/b/c/" — should walk up and find it in "a/"
        let cfg = TestGapConfig::load(&c);
        assert_eq!(cfg.min_severity, GapSeverity::Critical);
    }

    // ── generate_default_config round-trips ─────────────────────────

    #[test]
    fn generate_default_config_round_trips() {
        let toml_text = generate_default_config();
        let parsed: TestGapConfig =
            toml::from_str(&toml_text).expect("generated config should parse");

        // Verify a few known values survive the round-trip
        assert_eq!(parsed.format, OutputFormat::Human);
        assert!(parsed.ai.enabled);
        assert_eq!(parsed.ai.model, "claude-sonnet-4-20250514");
        assert_eq!(parsed.ai.batch_size, 5);
        assert_eq!(parsed.min_severity, GapSeverity::Info);
        assert!(parsed.exclude.contains(&"**/target/**".to_string()));
    }
}
