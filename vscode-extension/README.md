# Cargo Fast Lint - VSCode Extension

Lightning-fast Rust linter with real-time feedback for VSCode.

## Features

- **Real-time linting** without compilation
- **Auto-fix suggestions** for common issues  
- **Fast analysis** - under 1 second on large codebases
- **Configurable rules** via `.fl.toml`
- **Zero dependencies** - pure AST analysis

## Installation

1. Install the `cargo-fl` tool:
   ```bash
   cargo install cargo-fl
   ```

2. Install this extension from the VSCode marketplace

## Configuration

Configure the extension in your VSCode settings:

- `cargo-fl.enable`: Enable/disable the linter
- `cargo-fl.executablePath`: Path to cargo-fl-lsp executable  
- `cargo-fl.autoFix`: Automatically apply fixes when available
- `cargo-fl.trace.server`: LSP trace level for debugging

## Commands

- `Cargo FL: Restart Language Server` - Restart the LSP server
- `Cargo FL: Show Output` - Show the output channel

## Status

This extension provides real-time linting through the LSP protocol. Look for the "FL" indicator in the status bar when active.