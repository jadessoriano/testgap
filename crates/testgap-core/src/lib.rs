pub mod ai_reasoner;
pub mod config;
pub mod function_extractor;
pub mod gap_detector;
pub mod git_diff;
pub mod language_registry;
pub mod reporter;
pub mod test_mapper;
pub mod types;

use config::TestGapConfig;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::Path;
use types::AnalysisReport;

#[derive(Debug, thiserror::Error)]
pub enum TestGapError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Parse error in {file}: {message}")]
    Parse { file: String, message: String },

    #[error("AI API error: {0}")]
    AiApi(String),

    #[error("No supported files found in {0}")]
    NoFiles(String),
}

pub type Result<T> = std::result::Result<T, TestGapError>;

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {msg}")
        .unwrap()
        .tick_chars("\u{25DC}\u{25DD}\u{25DE}\u{25DF}\u{2714} ")
}

fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {msg} [{bar:20.cyan/dim}] {pos}/{len}")
        .unwrap()
        .tick_chars("\u{25DC}\u{25DD}\u{25DE}\u{25DF}\u{2714} ")
        .progress_chars("\u{2588}\u{2591} ")
}

/// Run the full analysis pipeline on a project directory.
///
/// If `diff_base` is provided, only source files changed relative to that git ref
/// are analyzed. Test files are always included for complete test mapping.
pub async fn analyze(
    path: &Path,
    config: &TestGapConfig,
    diff_base: Option<&str>,
) -> Result<AnalysisReport> {
    let path = path.canonicalize().map_err(|e| {
        TestGapError::Io(std::io::Error::new(
            e.kind(),
            format!("{}: {e}", path.display()),
        ))
    })?;

    tracing::info!("Analyzing {}", path.display());

    let mp = MultiProgress::new();

    // Step 1: Scan
    let scan_spinner = mp.add(ProgressBar::new_spinner());
    scan_spinner.set_style(spinner_style());
    scan_spinner.set_message("Scanning\u{2026}");
    scan_spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let mut files = test_mapper::scan_directory(&path, config)?;
    if files.source_files.is_empty() {
        scan_spinner.finish_and_clear();
        return Err(TestGapError::NoFiles(path.display().to_string()));
    }

    // Filter source files to only changed files when diff mode is active
    if let Some(base) = diff_base {
        let changed = git_diff::get_changed_files(&path, base)?;
        let before = files.source_files.len();
        files.source_files.retain(|f| {
            // Match relative path against the changed set
            f.path
                .strip_prefix(&path)
                .ok()
                .map(|rel| changed.contains(rel))
                .unwrap_or(false)
        });
        tracing::info!(
            "Diff filter: {before} → {} source files (base: {base})",
            files.source_files.len()
        );
        if files.source_files.is_empty() {
            scan_spinner.finish_and_clear();
            eprintln!("No changed source files found relative to {base}.");
            return Ok(AnalysisReport {
                project_path: path,
                total_functions: 0,
                tested_functions: 0,
                gaps: vec![],
                languages_analyzed: vec![],
                ai_enabled: config.ai.enabled,
                token_usage: None,
                diff_base: diff_base.map(String::from),
            });
        }
    }

    scan_spinner.set_style(spinner_style());
    scan_spinner.finish_with_message(format!(
        "\u{2714} Scanned: {} source + {} test files",
        files.source_files.len(),
        files.test_files.len(),
    ));

    // Step 2: Extract functions
    let total_files = files.source_files.len() + files.test_files.len();
    let extract_bar = mp.add(ProgressBar::new(total_files as u64));
    extract_bar.set_style(bar_style());
    extract_bar.set_message("Extracting");

    let source_results: Vec<_> = files
        .source_files
        .par_iter()
        .map(|file| {
            let result = function_extractor::extract_functions(file);
            extract_bar.inc(1);
            (file, result)
        })
        .collect();

    let mut all_functions = Vec::new();
    let mut languages_seen = std::collections::HashSet::new();

    for (file, result) in source_results {
        match result {
            Ok(funcs) => {
                for f in &funcs {
                    languages_seen.insert(f.language);
                }
                all_functions.extend(funcs);
            }
            Err(e) => {
                tracing::warn!("Skipping {}: {e}", file.path.display());
            }
        }
    }

    let test_results: Vec<_> = files
        .test_files
        .par_iter()
        .map(|file| {
            let result = function_extractor::extract_functions(file);
            extract_bar.inc(1);
            (file, result)
        })
        .collect();

    let mut test_functions = Vec::new();
    for (file, result) in test_results {
        match result {
            Ok(funcs) => test_functions.extend(funcs),
            Err(e) => {
                tracing::warn!("Skipping test file {}: {e}", file.path.display());
            }
        }
    }

    extract_bar.finish_and_clear();

    // Step 3: Map tests to source functions
    let test_mapping = test_mapper::map_tests_to_functions(&all_functions, &test_functions);

    // Step 4: Detect gaps
    let mut gaps = gap_detector::detect_gaps(&all_functions, &test_mapping, config);

    // Step 5: AI analysis (optional)
    let mut token_usage = None;
    if config.ai.enabled {
        let ai_min = config.ai.ai_min_severity;
        let mut ai_gaps: Vec<_> = gaps.iter_mut().filter(|g| g.severity >= ai_min).collect();
        if !ai_gaps.is_empty() {
            let ai_bar = mp.add(ProgressBar::new(ai_gaps.len() as u64));
            ai_bar.set_style(bar_style());
            ai_bar.set_message("AI analysis");

            match ai_reasoner::analyze_gaps(&mut ai_gaps, config, Some(&ai_bar)).await {
                Ok(usage) => token_usage = Some(usage),
                Err(e) => {
                    tracing::warn!("AI analysis failed, continuing without it: {e}");
                }
            }
            ai_bar.finish_and_clear();
        }
    }

    let source_functions: Vec<_> = all_functions.iter().filter(|f| !f.is_test).collect();
    let total_functions = source_functions.len();
    let tested_functions = source_functions
        .iter()
        .filter(|f| test_mapping.contains(&test_mapper::function_key(f)))
        .count();

    // Clear all progress bars before report output
    mp.clear().ok();

    Ok(AnalysisReport {
        project_path: path,
        total_functions,
        tested_functions,
        gaps,
        languages_analyzed: languages_seen.into_iter().collect(),
        ai_enabled: config.ai.enabled,
        token_usage,
        diff_base: diff_base.map(String::from),
    })
}
