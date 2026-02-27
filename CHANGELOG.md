# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-02-27

### Added
- `--ai-severity` CLI flag to filter which gaps are sent to the AI, reducing API cost (default: `critical`)
- Progress indicators to stderr: "Scanning...", "Extracting N files...", "Found N gaps..."
- `.gitignore` support via `ignore` crate — respects `.gitignore` rules when scanning
- Exit code documentation in `--help` output
- Best Practices section in README (use cases, anti-patterns, tips)
- Parallel file parsing with `rayon` for faster analysis on multi-core machines
- `ai_min_severity` field in `[ai]` config section
- Cargo.toml workspace metadata (`description`, `keywords`, `categories`, `readme`)

### Fixed
- **UTF-8 panic**: `truncate_body` and `truncate` now use `char_indices()` to find safe boundaries instead of byte-slicing, preventing panics on multi-byte characters
- **Coverage math**: `tested_functions` is now computed from the coverage mapping set, not `total - gaps.len()`, which previously undercounted when gaps were filtered by severity
- **Python signature truncation**: uses `rfind(':')` on the first line only, avoiding truncation at colons inside type annotations
- **Silent config errors**: config parse failures now print to stderr via `eprintln!` instead of only logging to tracing (which requires `--verbose`)
- **Stacked attributes**: Rust `#[test]` detection now walks all preceding sibling attributes, not just the immediate one
- **dedup correctness**: functions are now sorted before `dedup_by`, ensuring non-adjacent duplicates are removed

### Changed
- HTTP client now uses a 60-second timeout instead of no timeout
- Replaced `walkdir` with `ignore` crate in `test_mapper` for `.gitignore` support
- Removed `tokio` from core runtime dependencies (moved to dev-dependencies only)

### Removed
- Dead `test_query()` function from `language_registry` (was unused)

## [0.1.0] - 2026-02-27

### Added
- Initial release
- Tree-sitter based function extraction for Rust, JavaScript, TypeScript, Python, Go
- Static test gap detection with severity classification (Critical, Warning, Info)
- AI-powered analysis via Claude API for risk assessment and test suggestions
- CLI with `analyze` and `init` subcommands
- JSON, Markdown, and human-readable output formats
- `.testgap.toml` configuration file support
- `--fail-on-critical` flag for CI integration
