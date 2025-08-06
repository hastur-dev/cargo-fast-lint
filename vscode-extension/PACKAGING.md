# VS Code Extension Packaging Instructions

This document explains how to package the Cargo Fast Lint VS Code extension as a `.vsix` file for distribution.

## Prerequisites

1. **Node.js and npm** installed on your system
2. **VS Code Extension Manager (vsce)** - will be installed automatically via npm

## Building the VSIX Package

### Step 1: Install Dependencies

Navigate to the extension directory and install dependencies:

```bash
cd vscode-extension
npm install
```

### Step 2: Build the Cargo LSP Server (Optional but Recommended)

The extension can use the native Rust LSP server for better performance:

```bash
cd ..
cargo build --release --bin cargo-fl-lsp
```

This creates the `cargo-fl-lsp` binary that the extension will automatically detect.

### Step 3: Package the Extension

Create the `.vsix` file:

```bash
cd vscode-extension
npm run package
```

This will generate a file like `cargo-fl-0.2.0.vsix` in the extension directory.

## Publishing to VS Code Marketplace

### Option 1: Upload Manually

1. Go to the [VS Code Marketplace Publisher Portal](https://marketplace.visualstudio.com/manage)
2. Sign in with your Microsoft account
3. Navigate to your publisher profile (or create one)
4. Click "New Extension" → "Upload Extension"
5. **Upload the `.vsix` file** (not the folder)

### Option 2: Publish via Command Line

```bash
# Login to VS Code Marketplace (one-time setup)
npx vsce login hastur-dev

# Publish directly
npm run publish
```

## Testing the Extension Locally

Before publishing, test the extension:

```bash
# Install the .vsix file locally
code --install-extension cargo-fl-0.2.0.vsix

# Or use VS Code UI:
# Ctrl+Shift+P → "Extensions: Install from VSIX..."
```

## File Structure

The packaged extension includes:
- `client.js` - Language client for VS Code integration  
- `server.js` - Language server implementing LSP protocol
- `rust-analyzer.js` - Bridge to cargo-fl binary
- `package.json` - Extension manifest
- `README.md` - Extension documentation

## What Gets Excluded

The `.vscodeignore` file excludes:
- Source files (`src/**`)
- Development files (tests, configs)
- Git history and temporary files
- Node modules and build artifacts

## Troubleshooting

**Error: "Publisher not found"**
- Create a publisher account at marketplace.visualstudio.com
- Update the `publisher` field in `package.json`

**Error: "Missing dependencies"**  
- Run `npm install` in the extension directory
- Ensure all dependencies in `package.json` are installed

**Extension not working**
- Verify the Rust LSP server is built: `cargo build --release --bin cargo-fl-lsp`
- Check VS Code Developer Console (Help → Toggle Developer Tools)

## Final Output

**Upload this file to the VS Code Marketplace:**
`vscode-extension/cargo-fl-0.2.0.vsix`

The `.vsix` file contains everything needed for the extension to run in VS Code with live LSP-powered linting.