use crate::cache::{AnalysisCache, FileMetadata};
use crate::rules::{Issue, Rule, RuleContext};
use crate::config::Config;
use ahash::AHashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use rayon::prelude::*;

pub struct IncrementalAnalyzer {
    config: Arc<Config>,
    rules: Vec<Box<dyn Rule>>,
    cache: AnalysisCache,
}

#[derive(Debug)]
pub struct IncrementalResults {
    pub new_issues: AHashMap<PathBuf, Vec<Issue>>,
    pub cached_issues: AHashMap<PathBuf, Vec<Issue>>,
    pub stats: IncrementalStats,
}

#[derive(Debug, Default)]
pub struct IncrementalStats {
    pub files_analyzed: usize,
    pub files_from_cache: usize,
    pub files_skipped: usize,
    pub cache_hit_rate: f64,
}

impl IncrementalAnalyzer {
    pub fn new(config: Config) -> Self {
        let rules = crate::rules::get_enabled_rules(&config);
        let cache_dir = config.cache.cache_dir.clone()
            .unwrap_or_else(|| std::env::temp_dir().join("cargo-fl"));
        let cache = AnalysisCache::new(cache_dir);
        
        Self {
            config: Arc::new(config),
            rules,
            cache,
        }
    }
    
    pub fn analyze_files(&mut self, files: Vec<PathBuf>) -> IncrementalResults {
        // Clean up stale cache entries first
        self.cache.cleanup_stale_entries();
        
        let mut files_to_analyze = Vec::new();
        let mut cached_issues = AHashMap::new();
        let mut stats = IncrementalStats::default();
        
        // Determine which files need analysis
        for file_path in files {
            match self.cache.is_file_changed(&file_path) {
                Ok(true) => {
                    files_to_analyze.push(file_path);
                }
                Ok(false) => {
                    // File unchanged, use cached results
                    if let Some(cached) = self.cache.get_cached_analysis(&file_path) {
                        cached_issues.insert(file_path, cached.issues.clone());
                        stats.files_from_cache += 1;
                    } else {
                        // Cache miss, need to analyze
                        files_to_analyze.push(file_path);
                    }
                }
                Err(_) => {
                    // Error checking file, skip it
                    stats.files_skipped += 1;
                }
            }
        }
        
        // Analyze changed files in parallel
        let new_issues_vec: Vec<(PathBuf, Vec<Issue>)> = files_to_analyze
            .par_iter()
            .filter_map(|file_path| {
                match self.analyze_single_file(file_path) {
                    Ok(issues) => {
                        Some((file_path.clone(), issues))
                    }
                    Err(e) => {
                        eprintln!("Error analyzing {}: {}", file_path.display(), e);
                        None
                    }
                }
            })
            .collect();
        
        let new_issues: AHashMap<PathBuf, Vec<Issue>> = new_issues_vec.into_iter().collect();
        
        // Update cache with new results
        for (path, issues) in &new_issues {
            if let Err(e) = self.cache.store_analysis(path.clone(), issues.clone(), None) {
                eprintln!("Warning: Failed to cache results for {}: {}", path.display(), e);
            }
        }
        
        stats.files_analyzed = new_issues.len();
        let total_processed = stats.files_analyzed + stats.files_from_cache;
        stats.cache_hit_rate = if total_processed > 0 {
            (stats.files_from_cache as f64) / (total_processed as f64) * 100.0
        } else {
            0.0
        };
        
        IncrementalResults {
            new_issues,
            cached_issues,
            stats,
        }
    }
    
    fn analyze_single_file(&self, file_path: &Path) -> Result<Vec<Issue>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(file_path)?;
        let syntax_tree = syn::parse_file(&content)?;
        
        let mut ctx = RuleContext::new(
            file_path.to_path_buf(),
            content,
            syntax_tree,
        );
        
        // Apply each rule
        for rule in &self.rules {
            rule.check(&mut ctx);
        }
        
        Ok(ctx.issues)
    }
    
    pub fn invalidate_file(&mut self, path: &Path) {
        self.cache.remove_file(path);
    }
    
    pub fn get_cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.cache_stats()
    }
    
    pub fn save_cache(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache.save()
    }
}

impl IncrementalResults {
    pub fn all_issues(&self) -> AHashMap<PathBuf, Vec<Issue>> {
        let mut all_issues = self.new_issues.clone();
        all_issues.extend(self.cached_issues.clone());
        all_issues
    }
    
    pub fn total_issues(&self) -> usize {
        self.new_issues.values().map(|issues| issues.len()).sum::<usize>() +
        self.cached_issues.values().map(|issues| issues.len()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_incremental_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();
        let mut analyzer = IncrementalAnalyzer::new(config);
        
        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn test() { println!(\"hello\"); }").unwrap();
        
        // First analysis - should analyze the file
        let results1 = analyzer.analyze_files(vec![test_file.clone()]);
        assert_eq!(results1.stats.files_analyzed, 1);
        assert_eq!(results1.stats.files_from_cache, 0);
        
        // Second analysis - should use cache
        let results2 = analyzer.analyze_files(vec![test_file.clone()]);
        assert_eq!(results2.stats.files_analyzed, 0);
        assert_eq!(results2.stats.files_from_cache, 1);
        
        // Modify file and analyze again - should analyze the file
        fs::write(&test_file, "fn test() { println!(\"world\"); }").unwrap();
        let results3 = analyzer.analyze_files(vec![test_file.clone()]);
        assert_eq!(results3.stats.files_analyzed, 1);
        assert_eq!(results3.stats.files_from_cache, 0);
    }
}