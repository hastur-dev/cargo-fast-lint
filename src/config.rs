use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub rules: RuleConfig,
    pub style: StyleConfig,
    pub complexity: ComplexityConfig,
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
            ignore: vec![
                "target/**".to_string(),
                ".git/**".to_string(),
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