use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;
use testgap_core::config::{self, OutputFormat, TestGapConfig};
use testgap_core::reporter::ColorMode;
use testgap_core::types::GapSeverity;

#[derive(Parser)]
#[command(
    name = "testgap",
    version,
    about = "AI-powered test gap finder",
    after_help = "Exit codes:\n  0  Success (no critical gaps, or --fail-on-critical not set)\n  1  Critical gaps found (with --fail-on-critical)\n  2  Runtime error"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a project for untested functions
    Analyze {
        /// Path to the project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format
        #[arg(short, long)]
        format: Option<CliFormat>,

        /// Exit with code 1 if critical gaps are found
        #[arg(long)]
        fail_on_critical: bool,

        /// Disable AI analysis
        #[arg(long)]
        no_ai: bool,

        /// Restrict to specific languages (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        languages: Option<Vec<CliLanguage>>,

        /// Minimum severity to report
        #[arg(long)]
        min_severity: Option<CliSeverity>,

        /// Minimum gap severity to send to AI (reduces API cost)
        #[arg(long, default_value = "critical")]
        ai_severity: CliSeverity,

        /// Color output mode
        #[arg(long, default_value = "auto")]
        color: CliColorMode,

        /// Only analyze files changed relative to a git ref.
        /// Defaults to the remote default branch when used without a value.
        #[arg(long, num_args = 0..=1, default_missing_value = "@auto")]
        diff: Option<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create a default .testgap.toml config file
    Init {
        /// Overwrite existing config
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum CliFormat {
    Human,
    Json,
    Markdown,
    Sarif,
    Github,
}

#[derive(Clone, ValueEnum)]
enum CliLanguage {
    Rust,
    Javascript,
    Typescript,
    Python,
    Go,
}

#[derive(Clone, ValueEnum)]
enum CliSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, ValueEnum)]
enum CliColorMode {
    Auto,
    Always,
    Never,
}

impl From<CliFormat> for OutputFormat {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Human => OutputFormat::Human,
            CliFormat::Json => OutputFormat::Json,
            CliFormat::Markdown => OutputFormat::Markdown,
            CliFormat::Sarif => OutputFormat::Sarif,
            CliFormat::Github => OutputFormat::Github,
        }
    }
}

impl From<CliLanguage> for testgap_core::types::Language {
    fn from(l: CliLanguage) -> Self {
        match l {
            CliLanguage::Rust => Self::Rust,
            CliLanguage::Javascript => Self::JavaScript,
            CliLanguage::Typescript => Self::TypeScript,
            CliLanguage::Python => Self::Python,
            CliLanguage::Go => Self::Go,
        }
    }
}

impl From<CliSeverity> for GapSeverity {
    fn from(s: CliSeverity) -> Self {
        match s {
            CliSeverity::Info => GapSeverity::Info,
            CliSeverity::Warning => GapSeverity::Warning,
            CliSeverity::Critical => GapSeverity::Critical,
        }
    }
}

impl From<CliColorMode> for ColorMode {
    fn from(c: CliColorMode) -> Self {
        match c {
            CliColorMode::Auto => ColorMode::Auto,
            CliColorMode::Always => ColorMode::Always,
            CliColorMode::Never => ColorMode::Never,
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            path,
            format,
            fail_on_critical,
            no_ai,
            languages,
            min_severity,
            ai_severity,
            diff,
            color,
            verbose,
        } => {
            init_tracing(verbose);
            run_analyze(AnalyzeArgs {
                path,
                format,
                fail_on_critical,
                no_ai,
                languages,
                min_severity,
                ai_severity,
                diff,
                color: color.into(),
            })
            .await
        }
        Commands::Init { force } => {
            init_tracing(false);
            run_init(force)
        }
    }
}

fn init_tracing(verbose: bool) {
    use tracing_subscriber::EnvFilter;
    let filter = if verbose {
        EnvFilter::new("testgap=debug")
    } else {
        EnvFilter::new("testgap=warn")
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

struct AnalyzeArgs {
    path: PathBuf,
    format: Option<CliFormat>,
    fail_on_critical: bool,
    no_ai: bool,
    languages: Option<Vec<CliLanguage>>,
    min_severity: Option<CliSeverity>,
    ai_severity: CliSeverity,
    diff: Option<String>,
    color: ColorMode,
}

async fn run_analyze(args: AnalyzeArgs) -> ExitCode {
    let mut config = TestGapConfig::load(&args.path);
    config.merge_cli_overrides(
        args.format.map(Into::into),
        args.languages
            .map(|v| v.into_iter().map(Into::into).collect()),
        args.min_severity.map(Into::into),
        args.no_ai,
        Some(args.ai_severity.into()),
    );

    // Resolve "@auto" sentinel to the actual default branch
    let diff_ref = match args.diff.as_deref() {
        Some("@auto") => match testgap_core::git_diff::resolve_default_branch(&args.path) {
            Ok(branch) => Some(branch),
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::from(2);
            }
        },
        other => other.map(String::from),
    };

    match testgap_core::analyze(&args.path, &config, diff_ref.as_deref()).await {
        Ok(report) => {
            testgap_core::reporter::print_report(&report, config.format, args.color);
            if args.fail_on_critical && report.has_critical_gaps() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run_init(force: bool) -> ExitCode {
    let path = std::env::current_dir()
        .unwrap_or_default()
        .join(".testgap.toml");
    if path.exists() && !force {
        eprintln!(
            "Config file already exists: {}\nUse --force to overwrite.",
            path.display()
        );
        return ExitCode::from(2);
    }
    match std::fs::write(&path, config::generate_default_config()) {
        Ok(()) => {
            println!("Created {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to write config: {e}");
            ExitCode::from(2)
        }
    }
}
