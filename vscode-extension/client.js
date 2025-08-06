const path = require('path');
const vscode = require('vscode');
const {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} = require('vscode-languageclient/node');

let client;

function activate(context) {
    // The server is implemented in node
    const serverModule = context.asAbsolutePath(path.join('server.js'));
    
    // The debug options for the server
    const debugOptions = { execArgv: ['--nolazy', '--inspect=6009'] };

    // If the extension is launched in debug mode then the debug server options are used
    // Otherwise the run options are used
    const serverOptions = {
        run: { module: serverModule, transport: TransportKind.ipc },
        debug: {
            module: serverModule,
            transport: TransportKind.ipc,
            options: debugOptions
        }
    };

    // Options to control the language client
    const clientOptions = {
        // Register the server for Rust documents
        documentSelector: [{ scheme: 'file', language: 'rust' }],
        synchronize: {
            // Notify the server about file changes to '.rs files contained in the workspace
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.rs')
        }
    };

    // Create the language client and start the client.
    client = new LanguageClient(
        'cargoFl',
        'Cargo Fast Lint Language Server',
        serverOptions,
        clientOptions
    );

    // Register commands
    const restartCommand = vscode.commands.registerCommand('cargo-fl.restart', () => {
        client.restart();
    });

    context.subscriptions.push(restartCommand);

    // Start the client. This will also launch the server
    client.start();
}

function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

module.exports = {
    activate,
    deactivate
};