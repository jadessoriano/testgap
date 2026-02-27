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
    println!(
        "  Project:   {}",
        report.project_path.display()
    );
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
        if report.ai_enabled { "enabled" } else { "disabled" }
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
        println!(
            "  CRITICAL ({}) ─────────────────────────",
            critical.len()
        );
        for gap in &critical {
            print_gap_human(gap);
        }
        println!();
    }

    if !warnings.is_empty() {
        println!(
            "  WARNING ({}) ──────────────────────────",
            warnings.len()
        );
        for gap in &warnings {
            print_gap_human(gap);
        }
        println!();
    }

    if !info.is_empty() {
        println!(
            "  INFO ({}) ─────────────────────────────",
            info.len()
        );
        for gap in &info {
            print_gap_human(gap);
        }
        println!();
    }

    // Summary
    println!("  Summary: {} critical, {} warning, {} info",
        critical.len(), warnings.len(), info.len()
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
    println!(
        "    {} {}:{}",
        f.name,
        f.file_path.display(),
        f.line_start
    );
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
        if report.ai_enabled { "enabled" } else { "disabled" }
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
