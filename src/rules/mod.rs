use crate::config::Config;
use std::path::PathBuf;
use syn::{File, visit::Visit};

mod syntax;
mod style;
mod imports;
mod unsafe_code;
mod complexity;
mod docs;

pub use syntax::*;
pub use style::*;
pub use imports::*;
pub use unsafe_code::*;
pub use complexity::*;
pub use docs::*;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Issue {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub location: Location,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Fix {
    pub description: String,
    pub replacements: Vec<Replacement>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Replacement {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

pub struct RuleContext {
    pub file_path: PathBuf,
    pub content: String,
    pub syntax_tree: File,
    pub issues: Vec<Issue>,
}

pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, ctx: &mut RuleContext);
}

impl RuleContext {
    pub fn new(file_path: PathBuf, content: String, syntax_tree: File) -> Self {
        Self {
            file_path,
            content,
            syntax_tree,
            issues: Vec::new(),
        }
    }
    
    pub fn report(&mut self, issue: Issue) {
        self.issues.push(issue);
    }
    
    pub fn line_col(&self, _span: proc_macro2::Span) -> (usize, usize) {
        // For now, return line 1, column 1 as a fallback
        // In a real implementation, we'd need to track spans properly
        (1, 1)
    }
}

impl Severity {
    pub fn github_level(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "notice",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

impl Issue {
    pub fn display(&self) -> String {
        use colored::*;
        
        let severity_str = match self.severity {
            Severity::Error => "error".red().bold(),
            Severity::Warning => "warning".yellow().bold(),
            Severity::Info => "info".cyan().bold(),
        };
        
        format!(
            "  {}:{} {} [{}] {}",
            self.location.line,
            self.location.column,
            severity_str,
            self.rule.dimmed(),
            self.message
        )
    }
}

pub fn get_enabled_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = vec![];
    
    // Syntax rules - always enabled
    rules.push(Box::new(UnmatchedDelimitersRule));
    rules.push(Box::new(InvalidSyntaxRule));
    
    // Style rules
    if config.rules.check_naming {
        rules.push(Box::new(NamingConventionRule));
    }
    
    if config.rules.check_line_length {
        rules.push(Box::new(LineLengthRule::new(config.style.max_line_length)));
    }
    
    // Import rules
    if config.rules.check_imports {
        rules.push(Box::new(ImportOrderRule));
        rules.push(Box::new(UnusedImportRule));
    }
    
    // Safety rules
    if config.rules.check_unsafe {
        rules.push(Box::new(UnsafeBlockRule));
    }
    
    // Complexity rules
    if config.rules.check_complexity {
        rules.push(Box::new(CyclomaticComplexityRule::new(
            config.complexity.max_cyclomatic
        )));
        rules.push(Box::new(CognitiveComplexityRule::new(
            config.complexity.max_cognitive
        )));
    }
    
    // Documentation rules
    if config.rules.check_missing_docs {
        rules.push(Box::new(MissingDocsRule));
    }
    
    rules
}