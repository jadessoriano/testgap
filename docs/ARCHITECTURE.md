# Architecture

## Overview

testgap is an AI-powered test gap finder built as a Rust workspace with two crates:

- **testgap-core** -- the core analysis library that handles parsing, mapping, gap detection, and reporting.
- **testgap-cli** -- the CLI binary that wires together configuration, user input, and the core library.

## Data Flow

```
┌─────────────┐    ┌──────────────────┐    ┌───────────────┐
│ scan_directory│───►│ extract_functions │───►│ map_tests_to_ │
│ (test_mapper) │    │ (function_extractor)│    │   functions   │
└─────────────┘    └──────────────────┘    └───────┬───────┘
                                                    │
┌─────────────┐    ┌──────────────────┐    ┌───────▼───────┐
│ print_report │◄───│  analyze_gaps    │◄───│  detect_gaps  │
│  (reporter)  │    │  (ai_reasoner)   │    │(gap_detector) │
└─────────────┘    └──────────────────┘    └───────────────┘
```

1. **Scan** -- `test_mapper` walks the target directory, classifying files as source or test files based on language conventions.
2. **Extract** -- `function_extractor` parses each file with tree-sitter and pulls out function definitions with metadata (name, visibility, signature, body, complexity).
3. **Map** -- `test_mapper` links tests to the functions they cover using name matching, Go-style prefixes, body references, and file conventions.
4. **Detect** -- `gap_detector` identifies functions without test coverage and classifies their severity.
5. **Analyze** -- `ai_reasoner` optionally sends untested functions to the Claude API for risk assessment and test suggestions.
6. **Report** -- `reporter` formats the final output in the requested format (human-readable, JSON, or markdown).

## Module Responsibilities

### types.rs
Core data types shared across the crate. Defines the `Language` enum, `ExtractedFunction`, `TestGap`, `GapSeverity`, and `AnalysisReport`. These types form the contract between pipeline stages.

### config.rs
Configuration loading from `.testgap.toml` with walk-up directory search (starting from the analyzed directory, searching upward toward the filesystem root). Uses TOML deserialization via serde with sensible defaults for all fields.

### language_registry.rs
Tree-sitter language and grammar setup. Handles file classification (test file vs source file) based on per-language naming conventions. Provides query definitions for function extraction, parameterized by language.

### function_extractor.rs
Tree-sitter parsing engine. Extracts functions with their name, visibility, signature, and body text. Estimates cyclomatic complexity by counting branch points in the function body. Handles language-specific visibility rules.

### test_mapper.rs
Scans directories for source and test files. Maps tests to functions via multiple strategies: exact name matching, Go-style `TestFunctionName` prefixes, body reference scanning, and file-level conventions (e.g., `foo_test.go` covers `foo.go`).

### gap_detector.rs
Compares extracted functions against the test coverage map to find untested functions. Classifies severity along two axes:
- **Critical** -- public functions with high complexity.
- **Warning** -- public functions with low complexity.
- **Info** -- private or internal functions.

Filters out trivial functions (e.g., simple getters) to reduce noise.

### reporter.rs
Three output formats:
- **Human-readable** -- colored terminal output with summary tables.
- **JSON** -- serde_json serialization of the full `AnalysisReport` struct.
- **Markdown** -- structured markdown suitable for CI comments or documentation.

### ai_reasoner.rs
Optional Claude API integration. Sends batched async requests with function signatures and context. Parses AI responses for risk assessment scores and concrete test suggestions. Designed to fail gracefully -- if the API is unavailable or no key is configured, the pipeline continues without AI analysis.

### lib.rs
Orchestrates the full pipeline: scan -> extract -> map -> detect -> AI analyze -> report. Provides the public `analyze()` entry point consumed by the CLI.

## Key Design Decisions

### Tree-sitter for parsing
Language-agnostic AST parsing avoids fragile per-language regex hacks. Adding support for a new language means adding a tree-sitter grammar dependency and writing the corresponding queries -- no changes to the core pipeline logic.

### Severity classification
A simple two-axis model (visibility x complexity) rather than ML-based scoring. This keeps the static analysis phase fast and deterministic. The AI layer can provide nuanced assessment on top when available.

### AI as optional layer
Core analysis works without API keys. The AI adds risk assessment and test suggestions as an enrichment step but never blocks the pipeline. This makes testgap usable in offline environments and CI systems without secrets.

### Walk-up config search
Configuration discovery mirrors how tools like `.gitignore` and `.editorconfig` work -- search upward from the analyzed directory until a `.testgap.toml` is found or the filesystem root is reached. This supports both monorepo and single-project layouts without extra flags.
