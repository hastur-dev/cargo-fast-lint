import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('cargo-fl');
    
    if (!config.get('enable')) {
        return;
    }

    const executablePath = config.get<string>('executablePath') || 'cargo-fl-lsp';
    
    // Server options - executable path and arguments
    const serverOptions: ServerOptions = {
        command: executablePath,
        args: [],
        transport: TransportKind.stdio,
    };

    // Language client options
    const clientOptions: LanguageClientOptions = {
        // Register the server for Rust files
        documentSelector: [{ scheme: 'file', language: 'rust' }],
        synchronize: {
            // Notify the server about file changes to '.fl.toml' files in the workspace
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.fl.toml')
        },
        outputChannelName: 'Cargo Fast Lint',
        traceOutputChannel: vscode.window.createOutputChannel('Cargo Fast Lint Trace'),
    };

    // Create the language client and start it
    client = new LanguageClient(
        'cargo-fl',
        'Cargo Fast Lint',
        serverOptions,
        clientOptions
    );

    // Register commands
    const restartCommand = vscode.commands.registerCommand('cargo-fl.restart', async () => {
        await client.stop();
        client.start();
        vscode.window.showInformationMessage('Cargo FL Language Server restarted');
    });

    const showOutputCommand = vscode.commands.registerCommand('cargo-fl.showOutput', () => {
        client.outputChannel?.show();
    });

    context.subscriptions.push(restartCommand, showOutputCommand);

    // Start the client and server
    client.start();

    // Status bar item
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.text = "$(check) FL";
    statusBarItem.tooltip = "Cargo Fast Lint is active";
    statusBarItem.command = 'cargo-fl.showOutput';
    statusBarItem.show();
    
    context.subscriptions.push(statusBarItem);

    // Watch for configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((event) => {
            if (event.affectsConfiguration('cargo-fl')) {
                vscode.window.showInformationMessage(
                    'Cargo FL settings changed. Restart the language server to apply changes.',
                    'Restart'
                ).then((selection) => {
                    if (selection === 'Restart') {
                        vscode.commands.executeCommand('cargo-fl.restart');
                    }
                });
            }
        })
    );

    console.log('Cargo Fast Lint extension activated');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}