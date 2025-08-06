use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use cargo_fl::analyzer::Analyzer;
use cargo_fl::config::Config;
use cargo_fl::rules::Severity;

pub struct Backend {
    client: Client,
    analyzer: Mutex<Analyzer>,
    config: Mutex<Config>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let config = Config::load_or_default(&PathBuf::from("."));
        let analyzer = Analyzer::new(config.clone());
        
        Self {
            client,
            analyzer: Mutex::new(analyzer),
            config: Mutex::new(config),
        }
    }

    async fn lint_document(&self, uri: &Url) -> Result<Vec<Diagnostic>> {
        let path = uri.to_file_path().map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        
        let analyzer = self.analyzer.lock().await;
        let results = analyzer.analyze_file(&path);
        
        let mut diagnostics = Vec::new();
        
        if let Some(issues) = results.file_issues.get(&path) {
            for issue in issues {
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position {
                            line: issue.location.line.saturating_sub(1) as u32,
                            character: issue.location.column.saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: issue.location.line.saturating_sub(1) as u32,
                            character: issue.location.end_column.unwrap_or(issue.location.column + 1).saturating_sub(1) as u32,
                        },
                    },
                    severity: Some(match issue.severity {
                        Severity::Error => DiagnosticSeverity::ERROR,
                        Severity::Warning => DiagnosticSeverity::WARNING,
                        Severity::Info => DiagnosticSeverity::INFORMATION,
                    }),
                    code: Some(NumberOrString::String(issue.rule.to_string())),
                    source: Some("cargo-fl".to_string()),
                    message: issue.message.clone(),
                    related_information: None,
                    tags: None,
                    code_description: None,
                    data: None,
                };
                diagnostics.push(diagnostic);
            }
        }
        
        Ok(diagnostics)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "cargo-fl".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                    identifier: Some("cargo-fl".to_string()),
                    inter_file_dependencies: true,
                    workspace_diagnostics: false,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cargo-fl LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let diagnostics = self.lint_document(&params.text_document.uri).await.unwrap_or_default();
        
        self.client
            .publish_diagnostics(params.text_document.uri.clone(), diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let diagnostics = self.lint_document(&params.text_document.uri).await.unwrap_or_default();
        
        self.client
            .publish_diagnostics(params.text_document.uri.clone(), diagnostics, None)
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let diagnostics = self.lint_document(&params.text_document.uri).await.unwrap_or_default();
        
        self.client
            .publish_diagnostics(params.text_document.uri.clone(), diagnostics, None)
            .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let path = uri.to_file_path().map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        
        let analyzer = self.analyzer.lock().await;
        let results = analyzer.analyze_file(&path);
        
        let mut actions = Vec::new();
        
        if let Some(issues) = results.file_issues.get(&path) {
            for issue in issues {
                if issue.fix.is_some() {
                    let range = Range {
                        start: Position {
                            line: issue.location.line.saturating_sub(1) as u32,
                            character: issue.location.column.saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: issue.location.line.saturating_sub(1) as u32,
                            character: issue.location.end_column.unwrap_or(issue.location.column + 1).saturating_sub(1) as u32,
                        },
                    };

                    if params.range.start <= range.start && range.end <= params.range.end {
                        let fix = issue.fix.as_ref().unwrap();
                        let fix_text = &fix.description;
                        let action = CodeAction {
                            title: format!("Fix: {}", issue.message),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![Diagnostic {
                                range,
                                severity: Some(DiagnosticSeverity::WARNING),
                                code: Some(NumberOrString::String(issue.rule.to_string())),
                                source: Some("cargo-fl".to_string()),
                                message: issue.message.clone(),
                                related_information: None,
                                tags: None,
                                code_description: None,
                                data: None,
                            }]),
                            edit: Some(WorkspaceEdit {
                                changes: {
                                    let mut changes = HashMap::new();
                                    changes.insert(uri.clone(), vec![TextEdit {
                                        range,
                                        new_text: fix_text.clone(),
                                    }]);
                                    Some(changes)
                                },
                                document_changes: None,
                                change_annotations: None,
                            }),
                            command: None,
                            is_preferred: Some(true),
                            disabled: None,
                            data: None,
                        };
                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }
                }
            }
        }
        
        Ok(Some(actions))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}