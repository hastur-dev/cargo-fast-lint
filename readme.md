# cargo-fl

Lightning-fast Rust linter that runs **without compilation**. Similar to [ruff](https://github.com/astral-sh/ruff) for Python, but for Rust.

> `fl` = fast lint. Because developer time is precious.

## Features

- **< 1 second** on large codebases (100k+ lines)
- **Zero compilation** - pure AST analysis using `syn`
- **Parallel processing** with `rayon`
- **Auto-fix support** for common issues
- **Configurable rules** via `.fl.toml`

## Installation

```bash
cargo install cargo-fl