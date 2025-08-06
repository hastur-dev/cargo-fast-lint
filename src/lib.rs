//! cargo-fl: Lightning-fast Rust linter without compilation
//! 
//! This crate provides a high-performance linter for Rust code that operates
//! purely on the AST level without requiring compilation. It's designed to
//! catch common issues in under a second, even on large codebases.
//! 
//! `fl` = fast lint, because developer time is precious.

pub mod analyzer;
pub mod config;
pub mod rules;
pub mod walker;
pub mod cache;
pub mod incremental;
pub mod ast_cache;
pub mod autofix;

pub use analyzer::{Analyzer, AnalysisResults};
pub use config::{Config, ConfigManager};
pub use rules::{Issue, Severity, Rule};
pub use incremental::{IncrementalAnalyzer, IncrementalResults};
pub use cache::{AnalysisCache, CacheStats};
pub use autofix::{AutoFixEngine, ImportOrganizer, NamingConventionFixer, DocTemplateGenerator};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-exports for convenience
pub mod prelude {
    pub use crate::analyzer::Analyzer;
    pub use crate::config::Config;
    pub use crate::rules::{Issue, Severity};
}