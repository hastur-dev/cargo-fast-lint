use crate::config::Config;
use crate::rules::{Rule, RuleContext, Issue};
use crate::walker::RustFileWalker;
use ahash::AHashMap;
use dashmap::DashMap;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Analyzer {
    config: Arc<Config>,
    rules: Vec<Box<dyn Rule>>,
}

#[derive(Debug, serde::Serialize)]
pub struct AnalysisResults {
    pub file_issues: AHashMap<PathBuf, Vec<Issue>>,
    pub stats: AnalysisStats,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct AnalysisStats {
    pub total_files: usize,
    pub files_with_issues: usize,
    pub total_issues: usize,
    pub issues_by_severity: AHashMap<String, usize>,
}

impl Analyzer {
    pub fn new(config: Config) -> Self {
        let rules = crate::rules::get_enabled_rules(&config);
        Self {
            config: Arc::new(config),
            rules,
        }
    }
    
    pub fn analyze_path(&mut self, path: &Path) -> AnalysisResults {
        let walker = RustFileWalker::new();
        let files: Vec<_> = walker.walk(path).collect();
        
        let file_issues: DashMap<PathBuf, Vec<Issue>> = DashMap::new();
        
        // Process files in parallel
        files.par_iter().for_each(|file_path| {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Ok(syntax_tree) = syn::parse_file(&content) {
                    let mut ctx = RuleContext::new(
                        file_path.clone(),
                        content.clone(),
                        syntax_tree,
                    );
                    
                    // Apply each rule
                    for rule in &self.rules {
                        rule.check(&mut ctx);
                    }
                    
                    if !ctx.issues.is_empty() {
                        file_issues.insert(file_path.clone(), ctx.issues);
                    }
                }
            }
        });
        
        // Convert to regular HashMap and calculate stats
        let file_issues: AHashMap<_, _> = file_issues.into_iter().collect();
        let mut stats = AnalysisStats::default();
        
        stats.total_files = files.len();
        stats.files_with_issues = file_issues.len();
        
        for issues in file_issues.values() {
            stats.total_issues += issues.len();
            for issue in issues {
                *stats.issues_by_severity
                    .entry(issue.severity.to_string())
                    .or_insert(0) += 1;
            }
        }
        
        AnalysisResults { file_issues, stats }
    }

    pub fn analyze_file(&self, path: &Path) -> AnalysisResults {
        let mut file_issues = AHashMap::new();
        
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(syntax_tree) = syn::parse_file(&content) {
                let mut ctx = RuleContext::new(
                    path.to_path_buf(),
                    content.clone(),
                    syntax_tree,
                );
                
                // Apply each rule
                for rule in &self.rules {
                    rule.check(&mut ctx);
                }
                
                if !ctx.issues.is_empty() {
                    file_issues.insert(path.to_path_buf(), ctx.issues);
                }
            }
        }
        
        let mut stats = AnalysisStats::default();
        stats.total_files = 1;
        stats.files_with_issues = if file_issues.is_empty() { 0 } else { 1 };
        
        for issues in file_issues.values() {
            stats.total_issues += issues.len();
            for issue in issues {
                *stats.issues_by_severity
                    .entry(issue.severity.to_string())
                    .or_insert(0) += 1;
            }
        }
        
        AnalysisResults { file_issues, stats }
    }
}

impl AnalysisResults {
    pub fn total_issues(&self) -> usize {
        self.stats.total_issues
    }
    
    pub fn file_count(&self) -> usize {
        self.stats.total_files
    }
    
    pub fn files_with_issues(&self) -> usize {
        self.stats.files_with_issues
    }
    
    pub fn fixable_count(&self) -> usize {
        self.file_issues
            .values()
            .flat_map(|issues| issues.iter())
            .filter(|issue| issue.fix.is_some())
            .count()
    }
}