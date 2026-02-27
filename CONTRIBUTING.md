# Contributing to testgap

## Development Setup

### Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs/))

### Build and Verify

```bash
# Clone and build
cargo build --workspace

# Run tests
cargo test --workspace

# Check formatting
cargo fmt --all -- --check

# Lint
cargo clippy --workspace -- -D warnings
```

## Project Structure

```
testgap/
├── crates/
│   ├── testgap-core/       # Core analysis library
│   │   └── src/
│   │       ├── lib.rs             # Public API: analyze()
│   │       ├── types.rs           # Data types (Language, ExtractedFunction, TestGap, etc.)
│   │       ├── config.rs          # .testgap.toml config loading
│   │       ├── language_registry.rs # Tree-sitter language setup, file classification
│   │       ├── function_extractor.rs # Tree-sitter function extraction
│   │       ├── test_mapper.rs     # Test-to-function mapping
│   │       ├── gap_detector.rs    # Gap detection and severity classification
│   │       ├── reporter.rs        # Output formatting (human, JSON, markdown)
│   │       └── ai_reasoner.rs     # Claude API integration
│   └── testgap-cli/        # CLI binary
│       └── src/main.rs
├── .testgap.toml.example   # Example configuration
└── Cargo.toml              # Workspace definition
```

## Running Tests

- **Unit tests**: `cargo test --workspace`
- **Self-analysis**: `testgap analyze . --no-ai`

## How to Add a Language

1. Add a variant to the `Language` enum in `types.rs`.
2. Add extension mapping in `Language::from_extension()`.
3. Add tree-sitter dependency in `testgap-core/Cargo.toml`.
4. Add parser setup in `language_registry.rs`: `get_language()`, `function_query()`, `test_query()`.
5. Handle visibility in `function_extractor.rs`: `check_visibility()`, `check_is_test()`.
6. Add test file conventions if needed.

## Code Style

- Run `cargo fmt` before committing.
- All public items should pass `cargo clippy -- -D warnings`.
- Add `#[cfg(test)]` unit tests for new logic.
