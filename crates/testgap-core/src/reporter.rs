use crate::config::OutputFormat;
use crate::types::{AnalysisReport, GapSeverity, TestGap};

pub fn print_report(report: &AnalysisReport, format: OutputFormat) {
    match format {
        OutputFormat::Human => print_human(report),
        OutputFormat::Json => print_json(report),
        OutputFormat::Markdown => print_markdown(report),
    }
}

fn print_human(report: &AnalysisReport) {
    println!();
    println!("  testgap — Test Gap Analysis");
    println!("  {}", "─".repeat(40));
    println!("  Project:   {}", report.project_path.display());
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
        "  Coverage:  {}/{} functions ({:.1}%)",
        report.tested_functions,
        report.total_functions,
        report.coverage_percent()
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
        println!("  No test gaps found!");
        println!();
        return;
    }

    let critical = report.gaps_by_severity(GapSeverity::Critical);
    let warnings = report.gaps_by_severity(GapSeverity::Warning);
    let info = report.gaps_by_severity(GapSeverity::Info);

    if !critical.is_empty() {
        println!("  CRITICAL ({}) ─────────────────────────", critical.len());
        for gap in &critical {
            print_gap_human(gap);
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("  WARNING ({}) ──────────────────────────", warnings.len());
        for gap in &warnings {
            print_gap_human(gap);
        }
        println!();
    }

    if !info.is_empty() {
        println!("  INFO ({}) ─────────────────────────────", info.len());
        for gap in &info {
            print_gap_human(gap);
        }
        println!();
    }

    // Summary
    println!(
        "  Summary: {} critical, {} warning, {} info",
        critical.len(),
        warnings.len(),
        info.len()
    );

    if let Some(ref usage) = report.token_usage {
        println!(
            "  Tokens:  {} input, {} output",
            usage.input_tokens, usage.output_tokens
        );
    }
    println!();
}

fn print_gap_human(gap: &TestGap) {
    let f = &gap.function;
    println!("    {} {}:{}", f.name, f.file_path.display(), f.line_start);
    println!("      {}", gap.reason);
    println!("      Signature: {}", truncate(&f.signature, 80));
    println!("      Complexity: {}", f.complexity);

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
        "### `{}` — `{}:{}`",
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

#[cfg(test)]
mod tests {
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
        };

        let json = serde_json::to_string(&report).expect("should serialize empty report");
        let deserialized: AnalysisReport =
            serde_json::from_str(&json).expect("should deserialize empty report");

        assert_eq!(deserialized.total_functions, 0);
        assert_eq!(deserialized.gaps.len(), 0);
        assert_eq!(deserialized.coverage_percent(), 100.0);
    }
}
