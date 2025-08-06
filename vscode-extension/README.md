
# Cargo Fast Lint VS Code Extension

Lightning-fast Rust linter for VS Code with **live LSP integration**. This extension provides real-time linting as you type, using the `cargo-fl` analyzer engine.

## Features

- **Live linting** - Real-time diagnostics as you type
- **Ultra-fast analysis** - < 1 second on large codebases  
- **No compilation required** - Pure AST analysis using `syn`
- **Language Server Protocol** - Full LSP integration for seamless VS Code experience
- **Automatic fallback** - Works even without cargo-fl binary installed
- **Built-in rules** - Includes naming conventions, unsafe code detection, TODO tracking, and more

## How It Works

The extension includes a Language Server that:
1. **Primary Mode**: Uses the compiled `cargo-fl` binary from your project's `target/` directory
2. **Fallback Mode**: Built-in Rust analysis rules when binary is not available
3. **Live Updates**: Analyzes code as you type for immediate feedback

## Installation & Setup

1. Install this extension in VS Code
2. **Optional**: Build the cargo-fl binary for enhanced analysis:
   ```bash
   cd /path/to/cargo-fast-lint
   cargo build --release
   ```
3. Open any Rust file - the extension activates automatically

## Extension Settings

* `cargoFl.maxNumberOfProblems`: Maximum number of problems to report (default: 1000)
* `cargoFl.enableLinting`: Enable live linting (default: true)  
* `cargoFl.trace.server`: Debug server communication (default: "off")

## Commands

* `Cargo FL: Restart Language Server` - Restart the LSP server

## Built-in Rules

- **Naming Conventions**: Ensures proper snake_case for functions, PascalCase for structs
- **Line Length**: Warns about lines exceeding 100 characters
- **Unsafe Code Detection**: Highlights unsafe blocks
- **TODO Tracking**: Finds TODO/FIXME comments
- **Missing Documentation**: Suggests documentation for public items
- **Unused Variables**: Detects potentially unused variables

## Usage

1. Install the extension
2. Open a Rust file in VS Code
3. Start typing - diagnostics appear in real-time
4. Hover over underlined code for detailed messages
5. View all issues in the Problems panel (`Ctrl+Shift+M`)

## Development

The extension consists of:
- **Language Client** (`client.js`) - VS Code integration
- **Language Server** (`server.js`) - LSP protocol implementation  
- **Rust Analyzer Bridge** (`rust-analyzer.js`) - Connects to cargo-fl binary

## Release Notes

### 0.2.0

Live LSP integration with real-time linting and cargo-fl binary integration.
=======
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
