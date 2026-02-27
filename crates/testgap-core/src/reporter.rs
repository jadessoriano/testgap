use crate::config::OutputFormat;
use crate::types::{AnalysisReport, GapSeverity, TestGap};
use owo_colors::OwoColorize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub fn should_color(self) -> bool {
        match self {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                if std::env::var_os("NO_COLOR").is_some() {
                    return false;
                }
                supports_color::on(supports_color::Stream::Stdout).is_some()
            }
        }
    }
}

pub fn print_report(report: &AnalysisReport, format: OutputFormat, color: ColorMode) {
    match format {
        OutputFormat::Human => print_human(report, color.should_color()),
        OutputFormat::Json => print_json(report),
        OutputFormat::Markdown => print_markdown(report),
        OutputFormat::Sarif => print_sarif(report),
        OutputFormat::Github => print_github(report),
    }
}

fn coverage_bar(pct: f64, use_color: bool) -> String {
    const WIDTH: usize = 20;
    let filled = ((pct / 100.0) * WIDTH as f64).round() as usize;
    let filled = filled.min(WIDTH);
    let empty = WIDTH - filled;

    let bar_filled = "\u{2588}".repeat(filled);
    let bar_empty = "\u{2591}".repeat(empty);
    let pct_str = format!("{pct:.1}%");

    if use_color {
        format!(
            "[{}{}] {}",
            bar_filled.green(),
            bar_empty.dimmed(),
            pct_str.green().bold(),
        )
    } else {
        format!("[{bar_filled}{bar_empty}] {pct_str}")
    }
}

fn print_human(report: &AnalysisReport, use_color: bool) {
    println!();
    if use_color {
        println!(
            "  {} {}",
            "\u{25C8}".bold(),
            "testgap \u{2014} Test Gap Analysis".bold()
        );
    } else {
        println!("  testgap \u{2014} Test Gap Analysis");
    }
    println!("  {}", "\u{2500}".repeat(40));
    println!("  Project:   {}", report.project_path.display());
    if let Some(ref base) = report.diff_base {
        println!("  Diff base: {base}");
    }
    println!(
        "  Languages: {}",
        report
            .languages_analyzed
            .iter()
            .map(|l| l.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "  Coverage:  {}",
        coverage_bar(report.coverage_percent(), use_color),
    );
    println!(
        "  AI:        {}",
        if report.ai_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    if report.gaps.is_empty() {
        if use_color {
            println!("  {} No test gaps found!", "\u{2714}".green().bold());
        } else {
            println!("  No test gaps found!");
        }
        println!();
        return;
    }

    let critical = report.gaps_by_severity(GapSeverity::Critical);
    let warnings = report.gaps_by_severity(GapSeverity::Warning);
    let info = report.gaps_by_severity(GapSeverity::Info);

    if !critical.is_empty() {
        let header = format!("\u{2716} CRITICAL ({}) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", critical.len());
        if use_color {
            println!("  {}", header.red().bold());
        } else {
            println!("  {header}");
        }
        for gap in &critical {
            print_gap_human(gap, use_color);
        }
        println!();
    }

    if !warnings.is_empty() {
        let header = format!("\u{25B2} WARNING ({}) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", warnings.len());
        if use_color {
            println!("  {}", header.yellow().bold());
        } else {
            println!("  {header}");
        }
        for gap in &warnings {
            print_gap_human(gap, use_color);
        }
        println!();
    }

    if !info.is_empty() {
        let header = format!("\u{25CF} INFO ({}) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", info.len());
        if use_color {
            println!("  {}", header.dimmed());
        } else {
            println!("  {header}");
        }
        for gap in &info {
            print_gap_human(gap, use_color);
        }
        println!();
    }

    // Summary
    if use_color {
        println!(
            "  Summary: {} critical, {} warning, {} info",
            critical.len().to_string().red().bold(),
            warnings.len().to_string().yellow().bold(),
            info.len().to_string().dimmed(),
        );
    } else {
        println!(
            "  Summary: {} critical, {} warning, {} info",
            critical.len(),
            warnings.len(),
            info.len()
        );
    }

    if let Some(ref usage) = report.token_usage {
        println!(
            "  Tokens:  {} input, {} output",
            usage.input_tokens, usage.output_tokens
        );
    }
    println!();
}

fn print_gap_human(gap: &TestGap, use_color: bool) {
    let f = &gap.function;
    if use_color {
        println!(
            "    {} {}",
            f.name.bold(),
            format!("{}:{}", f.file_path.display(), f.line_start).dimmed(),
        );
    } else {
        println!("    {} {}:{}", f.name, f.file_path.display(), f.line_start);
    }
    println!("      {}", gap.reason);
    println!("      Signature: {}", truncate(&f.signature, 80));
    if use_color && f.complexity >= 5 {
        println!("      Complexity: {}", f.complexity.yellow());
    } else {
        println!("      Complexity: {}", f.complexity);
    }

    if let Some(ref ai) = gap.ai_analysis {
        println!("      AI Risk: {}", ai.risk_assessment);
        println!("      Priority: {}/10", ai.priority_score);
        if !ai.suggested_tests.is_empty() {
            println!("      Suggested tests:");
            for test in &ai.suggested_tests {
                println!("        - {test}");
            }
        }
    }
    println!();
}

fn print_json(report: &AnalysisReport) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize report: {e}"),
    }
}

fn print_markdown(report: &AnalysisReport) {
    println!("# Test Gap Analysis Report");
    println!();
    println!("**Project:** `{}`", report.project_path.display());
    if let Some(ref base) = report.diff_base {
        println!("**Diff base:** `{base}`");
    }
    println!(
        "**Languages:** {}",
        report
            .languages_analyzed
            .iter()
            .map(|l| l.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "**Coverage:** {}/{} functions ({:.1}%)",
        report.tested_functions,
        report.total_functions,
        report.coverage_percent()
    );
    println!(
        "**AI Analysis:** {}",
        if report.ai_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    if report.gaps.is_empty() {
        println!("No test gaps found!");
        return;
    }

    let critical = report.gaps_by_severity(GapSeverity::Critical);
    let warnings = report.gaps_by_severity(GapSeverity::Warning);
    let info = report.gaps_by_severity(GapSeverity::Info);

    if !critical.is_empty() {
        println!("## Critical ({count})", count = critical.len());
        println!();
        for gap in &critical {
            print_gap_markdown(gap);
        }
    }

    if !warnings.is_empty() {
        println!("## Warning ({count})", count = warnings.len());
        println!();
        for gap in &warnings {
            print_gap_markdown(gap);
        }
    }

    if !info.is_empty() {
        println!(
            "<details>\n<summary>Info ({count})</summary>\n",
            count = info.len()
        );
        for gap in &info {
            print_gap_markdown(gap);
        }
        println!("</details>");
    }

    if let Some(ref usage) = report.token_usage {
        println!();
        println!(
            "---\n*AI tokens used: {} input, {} output*",
            usage.input_tokens, usage.output_tokens
        );
    }
}

fn print_gap_markdown(gap: &TestGap) {
    let f = &gap.function;
    println!(
        "### `{}` \u{2014} `{}:{}`",
        f.name,
        f.file_path.display(),
        f.line_start
    );
    println!();
    println!("- **Severity:** {}", gap.severity);
    println!("- **Reason:** {}", gap.reason);
    println!("- **Signature:** `{}`", truncate(&f.signature, 100));
    println!("- **Complexity:** {}", f.complexity);

    if let Some(ref ai) = gap.ai_analysis {
        println!("- **AI Risk:** {}", ai.risk_assessment);
        println!("- **Priority:** {}/10", ai.priority_score);
        if !ai.suggested_tests.is_empty() {
            println!("- **Suggested tests:**");
            for test in &ai.suggested_tests {
                println!("  - {test}");
            }
        }
    }
    println!();
}

// ── SARIF output ──────────────────────────────────────────────────────

fn print_sarif(report: &AnalysisReport) {
    let sarif = build_sarif(report);
    match serde_json::to_string_pretty(&sarif) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize SARIF: {e}"),
    }
}

pub fn build_sarif(report: &AnalysisReport) -> serde_json::Value {
    let rules = serde_json::json!([
        {
            "id": "testgap/critical",
            "shortDescription": { "text": "Critical test gap" },
            "defaultConfiguration": { "level": "error" }
        },
        {
            "id": "testgap/warning",
            "shortDescription": { "text": "Warning test gap" },
            "defaultConfiguration": { "level": "warning" }
        },
        {
            "id": "testgap/info",
            "shortDescription": { "text": "Informational test gap" },
            "defaultConfiguration": { "level": "note" }
        }
    ]);

    let project_path = &report.project_path;

    let results: Vec<serde_json::Value> = report
        .gaps
        .iter()
        .map(|gap| {
            let (rule_id, level) = match gap.severity {
                GapSeverity::Critical => ("testgap/critical", "error"),
                GapSeverity::Warning => ("testgap/warning", "warning"),
                GapSeverity::Info => ("testgap/info", "note"),
            };

            let rel_path = make_relative(&gap.function.file_path, project_path);

            serde_json::json!({
                "ruleId": rule_id,
                "level": level,
                "message": {
                    "text": format!("Untested function `{}`: {}", gap.function.name, gap.reason)
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": rel_path,
                            "uriBaseId": "%SRCROOT%"
                        },
                        "region": {
                            "startLine": gap.function.line_start,
                            "endLine": gap.function.line_end
                        }
                    }
                }]
            })
        })
        .collect();

    serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "testgap",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules
                }
            },
            "results": results
        }]
    })
}

// ── GitHub annotations output ─────────────────────────────────────────

fn print_github(report: &AnalysisReport) {
    let project_path = &report.project_path;
    for gap in &report.gaps {
        println!("{}", format_github_line(gap, project_path));
    }
}

pub fn format_github_line(gap: &TestGap, project_path: &Path) -> String {
    let cmd = match gap.severity {
        GapSeverity::Critical => "error",
        GapSeverity::Warning => "warning",
        GapSeverity::Info => "notice",
    };

    let f = &gap.function;
    let rel_path = make_relative(&f.file_path, project_path);
    format!(
        "::{cmd} file={rel_path},line={line},endLine={end},title=Untested: {name}::{reason}",
        line = f.line_start,
        end = f.line_end,
        name = f.name,
        reason = gap.reason,
    )
}

/// Strip project_path prefix to produce a relative path string.
fn make_relative(abs: &Path, project_path: &Path) -> String {
    abs.strip_prefix(project_path)
        .unwrap_or(abs)
        .display()
        .to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let limit = max.saturating_sub(3);
        let boundary = s[..limit]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        format!("{}...", &s[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::path::PathBuf;

    fn make_gap(name: &str, severity: GapSeverity, complexity: u32) -> TestGap {
        TestGap {
            function: ExtractedFunction {
                name: name.to_string(),
                file_path: PathBuf::from("src/lib.rs"),
                line_start: 1,
                line_end: 20,
                signature: format!("pub fn {name}()"),
                body: "    some body code\n    more code\n    even more\n    end".to_string(),
                language: Language::Rust,
                is_public: true,
                is_test: false,
                complexity,
            },
            severity,
            reason: format!("Test reason for {name}"),
            ai_analysis: None,
        }
    }

    #[test]
    fn json_round_trip() {
        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/test-project"),
            total_functions: 10,
            tested_functions: 7,
            gaps: vec![
                make_gap("untested_critical", GapSeverity::Critical, 8),
                make_gap("untested_warning", GapSeverity::Warning, 3),
                make_gap("untested_info", GapSeverity::Info, 2),
            ],
            languages_analyzed: vec![Language::Rust, Language::TypeScript],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };

        let json = serde_json::to_string(&report).expect("should serialize to JSON");
        let deserialized: AnalysisReport =
            serde_json::from_str(&json).expect("should deserialize from JSON");

        assert_eq!(deserialized.total_functions, 10);
        assert_eq!(deserialized.tested_functions, 7);
        assert_eq!(deserialized.gaps.len(), 3);
        assert_eq!(deserialized.gaps[0].severity, GapSeverity::Critical);
        assert_eq!(deserialized.gaps[0].function.name, "untested_critical");
        assert_eq!(deserialized.gaps[1].severity, GapSeverity::Warning);
        assert_eq!(deserialized.gaps[2].severity, GapSeverity::Info);
        assert!(!deserialized.ai_enabled);
        assert!(deserialized.token_usage.is_none());
    }

    #[test]
    fn json_round_trip_with_ai_analysis() {
        let mut gap = make_gap("risky_func", GapSeverity::Critical, 10);
        gap.ai_analysis = Some(AiAnalysis {
            risk_assessment: "High risk due to complex branching".to_string(),
            suggested_tests: vec![
                "test boundary conditions".to_string(),
                "test error paths".to_string(),
            ],
            priority_score: 9,
            reasoning: "Multiple code paths untested".to_string(),
        });

        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/ai-project"),
            total_functions: 5,
            tested_functions: 2,
            gaps: vec![gap],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: true,
            token_usage: Some(TokenUsage {
                input_tokens: 1500,
                output_tokens: 300,
            }),
            diff_base: None,
        };

        let json = serde_json::to_string(&report).expect("should serialize");
        let deserialized: AnalysisReport = serde_json::from_str(&json).expect("should deserialize");

        assert!(deserialized.ai_enabled);
        assert!(deserialized.token_usage.is_some());
        let usage = deserialized.token_usage.unwrap();
        assert_eq!(usage.input_tokens, 1500);
        assert_eq!(usage.output_tokens, 300);

        let ai = deserialized.gaps[0].ai_analysis.as_ref().unwrap();
        assert_eq!(ai.priority_score, 9);
        assert_eq!(ai.suggested_tests.len(), 2);
    }

    #[test]
    fn json_does_not_panic_on_long_signature() {
        let mut gap = make_gap("long_sig_func", GapSeverity::Warning, 3);
        gap.function.signature = "a".repeat(500);

        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/long-sig"),
            total_functions: 1,
            tested_functions: 0,
            gaps: vec![gap],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };

        // Should not panic
        let json =
            serde_json::to_string(&report).expect("should serialize even with long signature");
        assert!(!json.is_empty());

        let deserialized: AnalysisReport = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized.gaps[0].function.signature.len(), 500);
    }

    #[test]
    fn coverage_percent_calculation() {
        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/cov"),
            total_functions: 10,
            tested_functions: 7,
            gaps: vec![],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };

        let pct = report.coverage_percent();
        assert!((pct - 70.0).abs() < 0.01, "expected ~70%, got {}", pct);
    }

    #[test]
    fn empty_report_json_round_trip() {
        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/empty"),
            total_functions: 0,
            tested_functions: 0,
            gaps: vec![],
            languages_analyzed: vec![],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };

        let json = serde_json::to_string(&report).expect("should serialize empty report");
        let deserialized: AnalysisReport =
            serde_json::from_str(&json).expect("should deserialize empty report");

        assert_eq!(deserialized.total_functions, 0);
        assert_eq!(deserialized.gaps.len(), 0);
        assert_eq!(deserialized.coverage_percent(), 100.0);
    }

    #[test]
    fn color_mode_never_disables_color() {
        assert!(!ColorMode::Never.should_color());
    }

    #[test]
    fn color_mode_always_enables_color() {
        assert!(ColorMode::Always.should_color());
    }

    #[test]
    fn coverage_bar_plain() {
        let bar = coverage_bar(50.0, false);
        assert!(bar.contains("["));
        assert!(bar.contains("]"));
        assert!(bar.contains("50.0%"));
    }

    #[test]
    fn coverage_bar_zero() {
        let bar = coverage_bar(0.0, false);
        assert!(bar.contains("0.0%"));
    }

    #[test]
    fn coverage_bar_hundred() {
        let bar = coverage_bar(100.0, false);
        assert!(bar.contains("100.0%"));
    }

    // ── SARIF tests ─────────────────────────────────────────────────

    #[test]
    fn sarif_schema_version() {
        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/sarif"),
            total_functions: 1,
            tested_functions: 0,
            gaps: vec![make_gap("func_a", GapSeverity::Critical, 5)],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };
        let sarif = build_sarif(&report);
        assert_eq!(sarif["version"], "2.1.0");
        assert!(sarif["$schema"]
            .as_str()
            .unwrap()
            .contains("sarif-schema-2.1.0"));
    }

    #[test]
    fn sarif_severity_mapping() {
        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/sarif"),
            total_functions: 3,
            tested_functions: 0,
            gaps: vec![
                make_gap("crit", GapSeverity::Critical, 5),
                make_gap("warn", GapSeverity::Warning, 3),
                make_gap("info_fn", GapSeverity::Info, 1),
            ],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };
        let sarif = build_sarif(&report);
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0]["level"], "error");
        assert_eq!(results[0]["ruleId"], "testgap/critical");
        assert_eq!(results[1]["level"], "warning");
        assert_eq!(results[1]["ruleId"], "testgap/warning");
        assert_eq!(results[2]["level"], "note");
        assert_eq!(results[2]["ruleId"], "testgap/info");
    }

    #[test]
    fn sarif_empty_gaps_empty_results() {
        let report = AnalysisReport {
            project_path: PathBuf::from("/tmp/sarif"),
            total_functions: 5,
            tested_functions: 5,
            gaps: vec![],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };
        let sarif = build_sarif(&report);
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert!(results.is_empty());
    }

    // ── GitHub annotations tests ────────────────────────────────────

    #[test]
    fn github_critical_format() {
        let gap = make_gap("risky_fn", GapSeverity::Critical, 8);
        let project = PathBuf::from("");
        let line = format_github_line(&gap, &project);
        assert!(
            line.starts_with("::error "),
            "expected ::error, got: {line}"
        );
        assert!(line.contains("file=src/lib.rs"));
        assert!(line.contains("title=Untested: risky_fn"));
    }

    #[test]
    fn github_warning_format() {
        let gap = make_gap("warn_fn", GapSeverity::Warning, 3);
        let project = PathBuf::from("");
        let line = format_github_line(&gap, &project);
        assert!(
            line.starts_with("::warning "),
            "expected ::warning, got: {line}"
        );
    }

    #[test]
    fn github_info_format() {
        let gap = make_gap("info_fn", GapSeverity::Info, 1);
        let project = PathBuf::from("");
        let line = format_github_line(&gap, &project);
        assert!(
            line.starts_with("::notice "),
            "expected ::notice, got: {line}"
        );
    }

    #[test]
    fn github_strips_absolute_path() {
        let mut gap = make_gap("func", GapSeverity::Critical, 5);
        gap.function.file_path = PathBuf::from("/home/user/project/src/lib.rs");
        let project = PathBuf::from("/home/user/project");
        let line = format_github_line(&gap, &project);
        assert!(
            line.contains("file=src/lib.rs"),
            "expected relative path, got: {line}"
        );
    }

    #[test]
    fn sarif_uses_relative_paths() {
        let mut gap = make_gap("func", GapSeverity::Critical, 5);
        gap.function.file_path = PathBuf::from("/home/user/project/src/lib.rs");
        let report = AnalysisReport {
            project_path: PathBuf::from("/home/user/project"),
            total_functions: 1,
            tested_functions: 0,
            gaps: vec![gap],
            languages_analyzed: vec![Language::Rust],
            ai_enabled: false,
            token_usage: None,
            diff_base: None,
        };
        let sarif = build_sarif(&report);
        let uri = sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
            ["artifactLocation"]["uri"]
            .as_str()
            .unwrap();
        assert_eq!(uri, "src/lib.rs", "SARIF should use relative paths");
    }
}
