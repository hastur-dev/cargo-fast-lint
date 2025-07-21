use cargo_fast_lint::{ConfigManager, FastLinter};
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Skip "cargo" if present (when called as cargo subcommand)
    let args = if args.len() > 1 && args[1] == "fast-lint" {
        &args[2..]
    } else if args.len() > 0 && args[0].ends_with("cargo-fast-lint") {
        &args[1..]
    } else {
        &args[1..]
    };

    if args.is_empty() {
        print_help();
        return;
    }

    let config_manager = match ConfigManager::new() {
        Ok(cm) => cm,
        Err(e) => {
            eprintln!("Failed to initialize config: {}", e);
            process::exit(1);
        }
    };

    match args[0].as_str() {
        "check" => {
            let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
            let format_first =
                args.contains(&"--format".to_string()) || args.contains(&"-f".to_string());

            let config = config_manager.load_config();
            let linter = FastLinter::new(config);

            match linter.lint_project(Path::new(path), format_first) {
                Ok(errors) => linter.display_errors(&errors),
                Err(e) => {
                    eprintln!("❌ Linting failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "fmt" => {
            if args.len() < 2 {
                eprintln!("Usage: cargo fast-lint fmt <formatter-name>");
                eprintln!("Example: cargo fast-lint fmt rustfmt-plus-plus");
                process::exit(1);
            }

            let formatter = &args[1];
            let mut config = config_manager.load_config();
            config.formatter = formatter.to_string();

            if let Err(e) = config_manager.save_config(&config) {
                eprintln!("Failed to save config: {}", e);
                process::exit(1);
            }

            println!("✅ Formatter set to: cargo {}", formatter);
        }
        "config" => {
            let config = config_manager.load_config();
            println!("Current configuration:");
            println!("  max_line_length: {}", config.max_line_length);
            println!("  max_nesting_depth: {}", config.max_nesting_depth);
            println!("  formatter: cargo {}", config.formatter);
            println!("  auto_format: {}", config.auto_format);
            println!("  fail_on_error: {}", config.fail_on_error);
            println!();
            println!("Enabled checks:");
            println!("  todos: {}", config.check_todos);
            println!(
                "  trailing_whitespace: {}",
                config.check_trailing_whitespace
            );
            println!("  missing_docs: {}", config.check_missing_docs);
            println!("  unused_imports: {}", config.check_unused_imports);
            println!("  naming_conventions: {}", config.check_naming_conventions);
            println!("  complexity: {}", config.check_complexity);
        }
        "set" => {
            if args.len() < 3 {
                eprintln!("Usage: cargo fast-lint set <key> <value>");
                eprintln!("Available keys: max_line_length, auto_format, fail_on_error, etc.");
                process::exit(1);
            }

            let key = &args[1];
            let value = &args[2];
            let mut config = config_manager.load_config();

            if key == "max_line_length" {
                match value.parse::<usize>() {
                    Ok(val) => config.max_line_length = val,
                    Err(_) => {
                        eprintln!("Invalid value for max_line_length: {}", value);
                        process::exit(1);
                    }
                }
            } else if key == "auto_format" {
                match value.parse::<bool>() {
                    Ok(val) => config.auto_format = val,
                    Err(_) => {
                        eprintln!("Invalid value for auto_format: {} (use true/false)", value);
                        process::exit(1);
                    }
                }
            } else if key == "fail_on_error" {
                match value.parse::<bool>() {
                    Ok(val) => config.fail_on_error = val,
                    Err(_) => {
                        eprintln!(
                            "Invalid value for fail_on_error: {} (use true/false)",
                            value
                        );
                        process::exit(1);
                    }
                }
            } else {
                eprintln!("Unknown config key: {}", key);
                process::exit(1);
            }

            if let Err(e) = config_manager.save_config(&config) {
                eprintln!("Failed to save config: {}", e);
                process::exit(1);
            }

            println!("✅ Set {} = {}", key, value);
        }
        "install-hook" => {
            install_pre_build_hook();
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            eprintln!("Unknown command: {}", args[0]);
            print_help();
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("cargo-fast-lint - High-performance Rust linter");
    println!();
    println!("USAGE:");
    println!("    cargo fast-lint <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    check [PATH]     Run linter on project (default: current directory)");
    println!("                     Use --format or -f to format before linting");
    println!("    fmt <FORMATTER>  Set the formatter to use (e.g., 'rustfmt-plus-plus')");
    println!("    config           Show current configuration");
    println!("    set <KEY> <VAL>  Set configuration value");
    println!("    install-hook     Install pre-build hook (experimental)");
    println!("    help             Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    cargo fast-lint check");
    println!("    cargo fast-lint check --format");
    println!("    cargo fast-lint fmt rustfmt-plus-plus");
    println!("    cargo fast-lint set max_line_length 120");
    println!("    cargo fast-lint set auto_format true");
}

fn install_pre_build_hook() {
    println!("🔧 Installing pre-build hook...");

    // This would create a build script or cargo alias
    // For now, we'll create a simple alias

    let cargo_config = r#"[alias]
lint-check = "fast-lint check"
"#;

    if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
        let cargo_dir = Path::new(&home).join(".cargo");
        let config_path = cargo_dir.join("config.toml");

        match std::fs::read_to_string(&config_path) {
            Ok(existing) => {
                if !existing.contains("lint-check") {
                    let updated = format!("{}\n{}", existing, cargo_config);
                    if std::fs::write(&config_path, updated).is_ok() {
                        println!("✅ Added 'cargo lint-check' alias");
                    }
                } else {
                    println!("✅ Alias already exists");
                }
            }
            Err(_) => {
                if std::fs::write(&config_path, cargo_config).is_ok() {
                    println!("✅ Created cargo config with 'cargo lint-check' alias");
                }
            }
        }
    }

    println!("You can now run: cargo lint-check");
}
