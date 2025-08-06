use crate::config::Config;
use crate::rules::{Rule, RuleContext, Issue};
use crate::walker::RustFileWalker;
use crate::incremental::{IncrementalAnalyzer, IncrementalResults};
use crate::ast_cache::{ASTCache, read_rust_file};
use crate::autofix::{AutoFixEngine, ImportOrganizer, NamingConventionFixer, DocTemplateGenerator};
use ahash::AHashMap;
use dashmap::DashMap;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Analyzer {
    config: Arc<Config>,
    rules: Vec<Box<dyn Rule>>,
    incremental_analyzer: Option<IncrementalAnalyzer>,
    ast_cache: Option<ASTCache>,
    autofix_engine: AutoFixEngine,
}

#[derive(Debug, serde::Serialize)]
pub struct AnalysisResults {
    pub file_issues: AHashMap<PathBuf, Vec<Issue>>,
    pub stats: AnalysisStats,
    pub performance_stats: Option<PerformanceStats>,
    pub fixed_files: Option<AHashMap<PathBuf, String>>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct AnalysisStats {
    pub total_files: usize,
    pub files_with_issues: usize,
    pub total_issues: usize,
    pub issues_by_severity: AHashMap<String, usize>,
}

#[derive(Debug, serde::Serialize)]
pub struct PerformanceStats {
    pub cache_hit_rate: f64,
    pub files_from_cache: usize,
    pub analysis_time_ms: u128,
    pub memory_usage_mb: Option<f64>,
    pub autofix_time_ms: Option<u128>,
    pub fixes_applied: usize,
}

impl Analyzer {
    pub fn new(config: Config) -> Self {
        let rules = crate::rules::get_enabled_rules(&config);
        
        // Initialize incremental analyzer if enabled
        let incremental_analyzer = if config.performance.incremental_analysis {
            Some(IncrementalAnalyzer::new(config.clone()))
        } else {
            None
        };
        
        // Initialize AST cache if enabled
        let ast_cache = if config.cache.ast_cache_enabled {
            let cache_dir = config.cache.cache_dir.clone()
                .unwrap_or_else(|| std::env::temp_dir().join("cargo-fl-ast"));
            Some(ASTCache::new(cache_dir))
        } else {
            None
        };
        
        Self {
            config: Arc::new(config),
            rules,
            incremental_analyzer,
            ast_cache,
            autofix_engine: AutoFixEngine::new(),
        }
    }
    
    pub fn analyze_path(&mut self, path: &Path) -> AnalysisResults {
        self.analyze_path_with_options(path, false)
    }
    
    pub fn analyze_path_with_autofix(&mut self, path: &Path) -> AnalysisResults {
        self.analyze_path_with_options(path, true)
    }
    
    fn analyze_path_with_options(&mut self, path: &Path, apply_autofix: bool) -> AnalysisResults {
        let start_time = std::time::Instant::now();
        
        let walker = RustFileWalker::new();
        let files: Vec<_> = walker.walk(path).collect();
        
        // Use incremental analysis if available
        let total_files = files.len();
        
        let (file_issues, mut performance_stats) = if let Some(ref mut incremental) = self.incremental_analyzer {
            let incremental_results = incremental.analyze_files(files);
            let all_issues = incremental_results.all_issues();
            
            let perf_stats = PerformanceStats {
                cache_hit_rate: incremental_results.stats.cache_hit_rate,
                files_from_cache: incremental_results.stats.files_from_cache,
                analysis_time_ms: start_time.elapsed().as_millis(),
                memory_usage_mb: None, // Could implement memory tracking
                autofix_time_ms: None,
                fixes_applied: 0,
            };
            
            (all_issues, Some(perf_stats))
        } else {
            // Fall back to traditional parallel analysis
            let file_issues = self.analyze_files_parallel(&files);
            let perf_stats = PerformanceStats {
                cache_hit_rate: 0.0,
                files_from_cache: 0,
                analysis_time_ms: start_time.elapsed().as_millis(),
                memory_usage_mb: None,
                autofix_time_ms: None,
                fixes_applied: 0,
            };
            
            (file_issues, Some(perf_stats))
        };
        
        // Apply auto-fixes if requested and enabled
        let mut fixed_files = None;
        let mut total_fixes_applied = 0;
        
        if apply_autofix && self.config.autofix.enabled {
            let autofix_start = std::time::Instant::now();
            let mut fixes = AHashMap::new();
            
            for (file_path, issues) in &file_issues {
                if let Ok(content) = read_rust_file(file_path) {
                    if let Ok(fixed_content) = self.autofix_engine.apply_fixes(&content, issues) {
                        if fixed_content != content {
                            fixes.insert(file_path.clone(), fixed_content);
                        }
                    }
                }
            }
            
            total_fixes_applied = self.autofix_engine.fixes_applied;
            if !fixes.is_empty() {
                fixed_files = Some(fixes);
            }
            
            // Update performance stats with autofix timing
            if let Some(ref mut perf_stats) = performance_stats.as_mut() {
                perf_stats.autofix_time_ms = Some(autofix_start.elapsed().as_millis());
                perf_stats.fixes_applied = total_fixes_applied;
            }
        }
        
        // Calculate regular stats
        let mut stats = AnalysisStats::default();
        stats.total_files = total_files;
        stats.files_with_issues = file_issues.len();
        
        for issues in file_issues.values() {
            stats.total_issues += issues.len();
            for issue in issues {
                *stats.issues_by_severity
                    .entry(issue.severity.to_string())
                    .or_insert(0) += 1;
            }
        }
        
        AnalysisResults { 
            file_issues, 
            stats,
            performance_stats,
            fixed_files,
        }
    }
    
    fn analyze_files_parallel(&self, files: &[PathBuf]) -> AHashMap<PathBuf, Vec<Issue>> {
        let file_issues: DashMap<PathBuf, Vec<Issue>> = DashMap::new();
        
        if self.config.performance.parallel_analysis {
            files.par_iter().for_each(|file_path| {
                if let Some(issues) = self.analyze_single_file(file_path) {
                    if !issues.is_empty() {
                        file_issues.insert(file_path.clone(), issues);
                    }
                }
            });
        } else {
            for file_path in files {
                if let Some(issues) = self.analyze_single_file(file_path) {
                    if !issues.is_empty() {
                        file_issues.insert(file_path.clone(), issues);
                    }
                }
            }
        }
        
        file_issues.into_iter().collect()
    }
    
    fn analyze_single_file(&self, file_path: &Path) -> Option<Vec<Issue>> {
        // Use memory-mapped reading for large files if enabled
        let content = if self.config.performance.memory_mapped_io {
            read_rust_file(file_path).ok()?
        } else {
            std::fs::read_to_string(file_path).ok()?
        };
        
        // Use AST cache if available
        let syntax_tree = if let Some(ref ast_cache) = self.ast_cache {
            ast_cache.get_or_parse(file_path).ok()?
        } else {
            syn::parse_file(&content).ok()?
        };
        
        let mut ctx = RuleContext::new(
            file_path.to_path_buf(),
            content,
            syntax_tree,
        );
        
        // Apply each rule
        for rule in &self.rules {
            rule.check(&mut ctx);
        }
        
        Some(ctx.issues)
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
        
        AnalysisResults { 
            file_issues, 
            stats,
            performance_stats: None,
            fixed_files: None,
        }
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
    
    pub fn cache_hit_rate(&self) -> f64 {
        self.performance_stats
            .as_ref()
            .map(|stats| stats.cache_hit_rate)
            .unwrap_or(0.0)
    }
    
    pub fn analysis_time_ms(&self) -> u128 {
        self.performance_stats
            .as_ref()
            .map(|stats| stats.analysis_time_ms)
            .unwrap_or(0)
    }
    
    pub fn fixes_applied(&self) -> usize {
        self.performance_stats
            .as_ref()
            .map(|stats| stats.fixes_applied)
            .unwrap_or(0)
    }
    
    pub fn has_fixes(&self) -> bool {
        self.fixed_files.is_some()
    }
}