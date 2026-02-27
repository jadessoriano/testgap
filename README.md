# testgap

[![CI](https://github.com/jadessoriano/testgap/actions/workflows/ci.yml/badge.svg)](https://github.com/jadessoriano/testgap/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

AI-powered test gap finder. Scans your codebase with tree-sitter, identifies untested functions, and uses Claude to suggest what tests to write.

## Install

**From crates.io** (once published):

```bash
cargo install testgap
```

**From source:**

```bash
git clone https://github.com/jadessoriano/testgap.git
cd testgap
cargo install --path crates/testgap-cli
```

## Quick Demo

```
$ testgap analyze --no-ai

  testgap — Test Gap Analysis
  ────────────────────────────────────────
  Project:   /home/user/my-project
  Languages: rust, typescript
  Coverage:  42/68 functions (61.8%)
  AI:        disabled

  CRITICAL (3) ─────────────────────────
    process_payment src/billing.rs:47
      Public function with high complexity (12) and no test coverage
      Signature: pub fn process_payment(order: &Order, method: PaymentMethod) -> Result<Receipt>
      Complexity: 12

    validate_schema src/api/handlers.rs:112
      Public function with high complexity (8) and no test coverage
      Signature: pub fn validate_schema(input: &Value, schema: &Schema) -> Result<()>
      Complexity: 8

  WARNING (5) ──────────────────────────
    ...

  Summary: 3 critical, 5 warning, 18 info
```

## Usage

```bash
# Analyze current directory
testgap analyze

# Static analysis only (no AI, no API key needed)
testgap analyze --no-ai

# Analyze a specific project
testgap analyze ./my-project

# JSON output for CI
testgap analyze --format json --fail-on-critical

# Markdown report
testgap analyze --format markdown

# Filter by language
testgap analyze --languages rust,typescript

# Only show critical gaps
testgap analyze --min-severity critical

# Create a config file
testgap init
```

## Supported Languages

| Language | Extensions | Test Detection |
|----------|-----------|----------------|
| Rust | `.rs` | `#[test]`, `#[cfg(test)]`, `test_` prefix |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | `test()`, `it()`, `describe()`, test dirs |
| TypeScript | `.ts`, `.tsx`, `.mts`, `.cts` | `test()`, `it()`, `describe()`, test dirs |
| Python | `.py` | `test_` prefix, test dirs |
| Go | `.go` | `Test` prefix, `*testing.T` parameter |

## How It Works

1. **Scan** — walks your project and classifies files as source or test
2. **Extract** — uses tree-sitter to parse functions from source files
3. **Map** — matches test functions to source functions by name, file convention, and body references
4. **Detect** — identifies untested functions and classifies severity:
   - **Critical**: public + complex + untested
   - **Warning**: public + untested
   - **Info**: private + untested
5. **Analyze** (optional) — sends gaps to Claude API for risk assessment and test suggestions

## Configuration

Create a `.testgap.toml` in your project root:

```bash
testgap init
```

See [.testgap.toml.example](.testgap.toml.example) for all options.

## CI Integration

```yaml
- name: Check test gaps
  run: testgap analyze --format json --fail-on-critical --no-ai
```

Exit codes:
- `0` — no critical gaps (or `--fail-on-critical` not set)
- `1` — critical gaps found (with `--fail-on-critical`)
- `2` — runtime error

## Environment Variables

- `ANTHROPIC_API_KEY` — required for AI analysis (use `--no-ai` to skip)

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the data flow diagram, module responsibilities, and key design decisions.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, running tests, and how to add a new language.

## License

MIT
