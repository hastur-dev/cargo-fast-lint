use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

mod analyzer;
mod config;
mod rules;
mod walker;

use analyzer::Analyzer;
use config::{Config, ConfigManager};

#[derive(Parser)]
#[command(name = "cargo-fl")]
#[command(bin_name = "cargo-fl")]
#[command(about = "Lightning-fast Rust linter (fl = fast lint)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run linter on project
    Check {
        /// Path to check (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        
        /// Fix auto-fixable issues
        #[arg(long, short)]
        fix: bool,
        
        /// Output format (default, json, github)
        #[arg(long, default_value = "default")]
        format: String,
        
        /// Exit with code 1 if any issues found
        #[arg(long)]
        strict: bool,
    },
    
    /// Show/modify configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,
        
        /// Generate default config file
        #[arg(long)]
        init: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    
    // Handle the cargo subcommand case
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "fl" {
        // Skip "cargo fl" and parse remaining args
        let cli = Cli::parse_from(&args[1..]);
        handle_command(cli);
    } else {
        handle_command(cli);
    }
}

fn handle_command(cli: Cli) {
    match cli.command {
        Commands::Check { path, fix, format, strict } => {
            run_check(path, fix, format, strict);
        }
        Commands::Config { show, init } => {
            handle_config(show, init);
        }
    }
}

fn run_check(path: PathBuf, fix: bool, format: String, strict: bool) {
    let start = Instant::now();
    
    // Load config
    let config = Config::load_or_default(&path);
    
    // Create analyzer
    let mut analyzer = Analyzer::new(config);
    
    // Walk files and analyze
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.set_message("Analyzing files...");
    
    let results = analyzer.analyze_path(&path);
    pb.finish_and_clear();
    
    // Display results
    let issue_count = results.total_issues();
    let file_count = results.file_count();
    let duration = start.elapsed();
    
    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results).unwrap());
        }
        "github" => {
            for (file, issues) in &results.file_issues {
                for issue in issues {
                    println!(
                        "::{}::file={},line={},col={}::{}",
                        issue.severity.github_level(),
                        file.display(),
                        issue.location.line,
                        issue.location.column,
                        issue.message
                    );
                }
            }
        }
        _ => {
            // Default format
            if issue_count == 0 {
                println!(
                    "{} {} files in {:.1}s",
                    "✓ Checked".green().bold(),
                    file_count,
                    duration.as_secs_f64()
                );
            } else {
                for (file, issues) in &results.file_issues {
                    if !issues.is_empty() {
                        println!("\n{}", file.display().to_string().bold());
                        for issue in issues {
                            println!("{}", issue.display());
                        }
                    }
                }
                
                println!(
                    "\n{} {} issues in {} files ({:.1}s)",
                    "Found".red().bold(),
                    issue_count,
                    results.files_with_issues(),
                    duration.as_secs_f64()
                );
                
                if fix && results.fixable_count() > 0 {
                    println!(
                        "{} {} issues can be fixed with --fix",
                        "→".yellow(),
                        results.fixable_count()
                    );
                }
            }
        }
    }
    
    if strict && issue_count > 0 {
        process::exit(1);
    }
}

fn handle_config(show: bool, init: bool) {
    let config_manager = ConfigManager::new();
    
    if init {
        config_manager.create_default_config().unwrap();
        println!("{} Created .fl.toml", "✓".green().bold());
    } else if show {
        let config = Config::load_or_default(&PathBuf::from("."));
        println!("{}", toml::to_string_pretty(&config).unwrap());
    }
}