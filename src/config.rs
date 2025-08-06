use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub rules: RuleConfig,
    pub style: StyleConfig,
    pub complexity: ComplexityConfig,
    pub cache: CacheConfig,
    pub autofix: AutoFixConfig,
    pub performance: PerformanceConfig,
    pub ignore: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleConfig {
    pub check_syntax: bool,
    pub check_style: bool,
    pub check_naming: bool,
    pub check_imports: bool,
    pub check_unsafe: bool,
    pub check_complexity: bool,
    pub check_missing_docs: bool,
    pub check_line_length: bool,
    pub check_unwrap_usage: bool,
    pub check_todo_macros: bool,
    pub check_must_use: bool,
    pub check_anti_patterns: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StyleConfig {
    pub max_line_length: usize,
    pub indent_size: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplexityConfig {
    pub max_cyclomatic: usize,
    pub max_cognitive: usize,
    pub max_nesting: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub cache_dir: Option<PathBuf>,
    pub ast_cache_enabled: bool,
    pub max_cache_size: usize,
    pub cache_ttl_hours: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoFixConfig {
    pub enabled: bool,
    pub organize_imports: bool,
    pub fix_naming_conventions: bool,
    pub add_missing_docs: bool,
    pub apply_safe_fixes_only: bool,
    pub max_fixes_per_file: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub incremental_analysis: bool,
    pub parallel_analysis: bool,
    pub memory_mapped_io: bool,
    pub large_file_threshold: usize,
    pub max_threads: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: RuleConfig {
                check_syntax: true,
                check_style: true,
                check_naming: true,
                check_imports: true,
                check_unsafe: true,
                check_complexity: true,
                check_missing_docs: true,
                check_line_length: true,
                check_unwrap_usage: true,
                check_todo_macros: true,
                check_must_use: true,
                check_anti_patterns: true,
            },
            style: StyleConfig {
                max_line_length: 100,
                indent_size: 4,
            },
            complexity: ComplexityConfig {
                max_cyclomatic: 10,
                max_cognitive: 15,
                max_nesting: 5,
            },
            cache: CacheConfig {
                enabled: true,
                cache_dir: None, // Will use system temp dir if None
                ast_cache_enabled: true,
                max_cache_size: 10000,
                cache_ttl_hours: 24,
            },
            autofix: AutoFixConfig {
                enabled: true,
                organize_imports: true,
                fix_naming_conventions: true,
                add_missing_docs: false, // Conservative default
                apply_safe_fixes_only: true,
                max_fixes_per_file: 100,
            },
            performance: PerformanceConfig {
                incremental_analysis: true,
                parallel_analysis: true,
                memory_mapped_io: true,
                large_file_threshold: 1024 * 1024, // 1MB
                max_threads: None, // Use all available cores
            },
            ignore: vec![
                "target/**".to_string(),
                ".git/**".to_string(),
                "node_modules/**".to_string(),
            ],
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Self {
        let config_path = path.join(".fl.toml");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn new() -> Self {
        Self
    }
    
    pub fn create_default_config(&self) -> std::io::Result<()> {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        std::fs::write(".fl.toml", toml)
    }
}