pub mod ai_reasoner;
pub mod config;
pub mod function_extractor;
pub mod gap_detector;
pub mod language_registry;
pub mod reporter;
pub mod test_mapper;
pub mod types;

use config::TestGapConfig;
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

/// Run the full analysis pipeline on a project directory.
pub async fn analyze(path: &Path, config: &TestGapConfig) -> Result<AnalysisReport> {
    let path = path.canonicalize().map_err(|e| {
        TestGapError::Io(std::io::Error::new(
            e.kind(),
            format!("{}: {e}", path.display()),
        ))
    })?;

    tracing::info!("Analyzing {}", path.display());

    // Step 1: Scan and extract functions
    eprintln!("Scanning...");
    let files = test_mapper::scan_directory(&path, config)?;
    if files.source_files.is_empty() {
        return Err(TestGapError::NoFiles(path.display().to_string()));
    }

    eprintln!(
        "Extracting {} source files + {} test files...",
        files.source_files.len(),
        files.test_files.len()
    );

    let source_results: Vec<_> = files
        .source_files
        .par_iter()
        .map(|file| (file, function_extractor::extract_functions(file)))
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
        .map(|file| (file, function_extractor::extract_functions(file)))
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

    // Step 2: Map tests to source functions
    let test_mapping = test_mapper::map_tests_to_functions(&all_functions, &test_functions);

    // Step 3: Detect gaps
    let mut gaps = gap_detector::detect_gaps(&all_functions, &test_mapping, config);

    // Step 4: AI analysis (optional)
    let mut token_usage = None;
    if config.ai.enabled {
        let ai_min = config.ai.ai_min_severity;
        let mut ai_gaps: Vec<_> = gaps.iter_mut().filter(|g| g.severity >= ai_min).collect();
        if !ai_gaps.is_empty() {
            match ai_reasoner::analyze_gaps(&mut ai_gaps, config).await {
                Ok(usage) => token_usage = Some(usage),
                Err(e) => {
                    tracing::warn!("AI analysis failed, continuing without it: {e}");
                }
            }
        }
    }

    let source_functions: Vec<_> = all_functions.iter().filter(|f| !f.is_test).collect();
    let total_functions = source_functions.len();
    let tested_functions = source_functions
        .iter()
        .filter(|f| test_mapping.contains(&test_mapper::function_key(f)))
        .count();

    eprintln!("Found {} gaps...", gaps.len());

    Ok(AnalysisReport {
        project_path: path,
        total_functions,
        tested_functions,
        gaps,
        languages_analyzed: languages_seen.into_iter().collect(),
        ai_enabled: config.ai.enabled,
        token_usage,
    })
}
