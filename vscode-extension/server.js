const {
    createConnection,
    TextDocuments,
    Diagnostic,
    DiagnosticSeverity,
    ProposedFeatures,
    InitializeParams,
    DidChangeConfigurationNotification,
    CompletionItem,
    CompletionItemKind,
    TextDocumentPositionParams,
    TextDocumentSyncKind,
    InitializeResult
} = require('vscode-languageserver/node');

const { TextDocument } = require('vscode-languageserver-textdocument');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Create a connection for the server
const connection = createConnection(ProposedFeatures.all);

// Create a simple text document manager
const documents = new TextDocuments(TextDocument);

let hasConfigurationCapability = false;
let hasWorkspaceFolderCapability = false;
let hasDiagnosticRelatedInformationCapability = false;

connection.onInitialize((params) => {
    const capabilities = params.capabilities;

    hasConfigurationCapability = !!(
        capabilities.workspace && !!capabilities.workspace.configuration
    );
    hasWorkspaceFolderCapability = !!(
        capabilities.workspace && !!capabilities.workspace.workspaceFolders
    );
    hasDiagnosticRelatedInformationCapability = !!(
        capabilities.textDocument &&
        capabilities.textDocument.publishDiagnostics &&
        capabilities.textDocument.publishDiagnostics.relatedInformation
    );

    const result = {
        capabilities: {
            textDocumentSync: TextDocumentSyncKind.Incremental,
            completionProvider: {
                resolveProvider: true
            }
        }
    };
    if (hasWorkspaceFolderCapability) {
        result.capabilities.workspace = {
            workspaceFolders: {
                supported: true
            }
        };
    }
    return result;
});

connection.onInitialized(() => {
    if (hasConfigurationCapability) {
        connection.client.register(DidChangeConfigurationNotification.type, undefined);
    }
    if (hasWorkspaceFolderCapability) {
        connection.workspace.onDidChangeWorkspaceFolders(_event => {
            connection.console.log('Workspace folder change event received.');
        });
    }
});

// Document settings
interface ExampleSettings {
    maxNumberOfProblems: number;
}

const defaultSettings = { maxNumberOfProblems: 1000 };
let globalSettings = defaultSettings;

// Cache the settings of all open documents
let documentSettings = new Map();

connection.onDidChangeConfiguration(change => {
    if (hasConfigurationCapability) {
        documentSettings.clear();
    } else {
        globalSettings = (change.settings.cargoFl || defaultSettings);
    }

    documents.all().forEach(validateTextDocument);
});

function getDocumentSettings(resource) {
    if (!hasConfigurationCapability) {
        return Promise.resolve(globalSettings);
    }
    let result = documentSettings.get(resource);
    if (!result) {
        result = connection.workspace.getConfiguration({
            scopeUri: resource,
            section: 'cargoFl'
        });
        documentSettings.set(resource, result);
    }
    return result;
}

// Only keep settings for open documents
documents.onDidClose(e => {
    documentSettings.delete(e.document.uri);
});

// The content of a text document has changed. This event is emitted
// when the text document first opened or when its content has changed.
documents.onDidChangeContent(change => {
    validateTextDocument(change.document);
});

async function validateTextDocument(textDocument) {
    const settings = await getDocumentSettings(textDocument.uri);
    const diagnostics = [];
    
    try {
        // Get file path from URI
        const filePath = textDocument.uri.replace('file://', '');
        
        // Check if this is a Rust file
        if (!filePath.endsWith('.rs')) {
            return;
        }

        // Use the cargo-fl analyzer logic directly
        const lintResults = analyzeRustCode(textDocument.getText(), filePath);
        
        for (const issue of lintResults) {
            const diagnostic = {
                severity: getSeverityFromString(issue.severity || 'warning'),
                range: {
                    start: textDocument.positionAt(issue.start || 0),
                    end: textDocument.positionAt(issue.end || issue.start || 0)
                },
                message: issue.message,
                source: 'cargo-fl'
            };
            
            if (issue.rule) {
                diagnostic.code = issue.rule;
            }
            
            if (hasDiagnosticRelatedInformationCapability && issue.relatedInformation) {
                diagnostic.relatedInformation = issue.relatedInformation;
            }
            
            diagnostics.push(diagnostic);
        }
    } catch (error) {
        connection.console.error(`Error analyzing ${textDocument.uri}: ${error.message}`);
    }

    // Send the computed diagnostics to VSCode
    connection.sendDiagnostics({ uri: textDocument.uri, diagnostics });
}

function getSeverityFromString(severity) {
    switch (severity.toLowerCase()) {
        case 'error':
            return DiagnosticSeverity.Error;
        case 'warning':
            return DiagnosticSeverity.Warning;
        case 'info':
            return DiagnosticSeverity.Information;
        case 'hint':
            return DiagnosticSeverity.Hint;
        default:
            return DiagnosticSeverity.Warning;
    }
}

const RustAnalyzer = require('./rust-analyzer');
const rustAnalyzer = new RustAnalyzer();

async function analyzeRustCode(content, filePath) {
    try {
        const issues = await rustAnalyzer.analyzeFile(filePath, content);
        
        // Convert cargo-fl format to LSP format
        return issues.map(issue => ({
            severity: issue.severity || 'warning',
            message: issue.message,
            rule: issue.rule,
            start: getOffsetFromLocation(content, issue.location),
            end: getEndOffsetFromLocation(content, issue.location)
        }));
    } catch (error) {
        connection.console.error(`Analysis error: ${error.message}`);
        return [];
    }
}

function getOffsetFromLocation(content, location) {
    if (!location) return 0;
    
    const lines = content.split('\n');
    let offset = 0;
    
    for (let i = 0; i < Math.min(location.line - 1, lines.length); i++) {
        offset += lines[i].length + 1; // +1 for newline
    }
    
    return offset + (location.column - 1);
}

function getEndOffsetFromLocation(content, location) {
    if (!location) return 0;
    
    if (location.end_line && location.end_column) {
        return getOffsetFromLocation(content, {
            line: location.end_line,
            column: location.end_column
        });
    }
    
    // Default to start + 1 if no end location
    return getOffsetFromLocation(content, location) + 1;
}

connection.onDidChangeWatchedFiles(_change => {
    connection.console.log('We received an file change event');
});

connection.onCompletion((_textDocumentPosition) => {
    return [
        {
            label: 'TypeScript',
            kind: CompletionItemKind.Text,
            data: 1
        },
        {
            label: 'JavaScript',
            kind: CompletionItemKind.Text,
            data: 2
        }
    ];
});

connection.onCompletionResolve((item) => {
    if (item.data === 1) {
        item.detail = 'TypeScript details';
        item.documentation = 'TypeScript documentation';
    } else if (item.data === 2) {
        item.detail = 'JavaScript details';
        item.documentation = 'JavaScript documentation';
    }
    return item;
});

// Make the text document manager listen on the connection
documents.listen(connection);

// Listen on the connection
connection.listen();