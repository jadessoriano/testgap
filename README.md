# testgap

AI-powered test gap finder. Scans your codebase with tree-sitter, identifies untested functions, and uses Claude to suggest what tests to write.

## Install

```bash
cargo install --path crates/testgap-cli
```

## Usage

```bash
# Analyze current directory
testgap analyze

# Analyze a specific project
testgap analyze ./my-project

# Static analysis only (no AI, no API key needed)
testgap analyze --no-ai

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

## Configuration

Create a `.testgap.toml` in your project root:

```bash
testgap init
```

See [.testgap.toml.example](.testgap.toml.example) for all options.

## How It Works

1. **Scan** — walks your project and classifies files as source or test
2. **Extract** — uses tree-sitter to parse functions from source files
3. **Map** — matches test functions to source functions by name, file convention, and body references
4. **Detect** — identifies untested functions and classifies severity:
   - **Critical**: public + complex + untested
   - **Warning**: public + untested
   - **Info**: private + untested
5. **Analyze** (optional) — sends gaps to Claude API for risk assessment and test suggestions

## Supported Languages

- Rust
- JavaScript / TypeScript
- Python
- Go

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

## License

MIT
