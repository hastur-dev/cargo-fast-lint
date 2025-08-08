# cargo-fl

Lightning-fast Rust linter that runs **without compilation**. Similar to [ruff](https://github.com/astral-sh/ruff) for Python, but for Rust.

> `fl` = fast lint. Because developer time is precious.

## Features

- **< 1 second** on large codebases (100k+ lines)
- **Zero compilation** - pure AST analysis using `syn`
- **Parallel processing** with `rayon`
- **Auto-fix support** for common issues
- **Configurable rules** via `.fl.toml`
- **LSP Server** for real-time editor feedback
- **VSCode Extension** for seamless integration

## Installation

```bash
cargo install cargo-fl
```

## Usage

### Command Line

```bash
# Lint current directory
cargo-fl check

# Lint specific path
cargo-fl check src/

# Auto-fix issues
cargo-fl check --fix

# Different output formats
cargo-fl check --format json
cargo-fl check --format github  # For CI

# Strict mode (exit 1 on issues)
cargo-fl check --strict
```

### Configuration

Generate a default config:
```bash
cargo-fl config --init
```

This creates `.fl.toml`:
```toml
[rules]
enable_all = true
disable = ["specific_rule"]

[severity]
unused_imports = "warning"
missing_docs = "info"
```

### Editor Integration

#### VSCode

1. Install the cargo-fl VSCode extension
2. The extension will automatically use the LSP server for real-time feedback

#### Other Editors

Use the LSP server directly:
```bash
cargo-fl-lsp
```

The LSP server supports:
- Real-time diagnostics
- Code actions (auto-fixes)
- Configuration via `.fl.toml`

## Rules

### Core Quality Rules
- **unwrap_usage**: Detects `.unwrap()`, `.unwrap_unchecked()`, and `.expect()` calls
- **todo_macros**: Finds `todo!()`, `unimplemented!()`, `unreachable!()`, and `panic!()` macros
- **must_use**: Identifies unused results from functions with `#[must_use]` return types
- **anti_patterns**: Common Rust anti-patterns and code smells

### Style & Structure Rules
- **unused_imports**: Detects unused import statements
- **missing_docs**: Missing documentation for public items
- **line_length**: Lines exceeding configured character limit
- **naming_convention**: Rust naming convention violations

### Complexity Rules
- **cyclomatic_complexity**: Functions with high cyclomatic complexity
- **cognitive_complexity**: Functions with high cognitive load
- **unsafe_code**: Unsafe code block warnings

### Import & Organization
- **import_order**: Incorrect import statement ordering
- **syntax_errors**: Basic syntax validation

## Performance

Typical performance on a 100k line Rust codebase:
- Analysis: < 1 second
- Memory usage: < 100MB
- Parallel processing across CPU cores

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Submit a pull request

## License

MIT OR Apache-2.0