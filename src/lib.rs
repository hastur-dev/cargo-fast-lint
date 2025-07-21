use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct LintError {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub rule: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct LintConfig {
    pub max_line_length: usize,
    pub max_nesting_depth: usize,
    pub check_todos: bool,
    pub check_trailing_whitespace: bool,
    pub check_missing_docs: bool,
    pub check_unused_imports: bool,
    pub check_naming_conventions: bool,
    pub check_complexity: bool,
    pub formatter: String,
    pub auto_format: bool,
    pub fail_on_error: bool,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            max_line_length: 100,
            max_nesting_depth: 6,
            check_todos: true,
            check_trailing_whitespace: true,
            check_missing_docs: true,
            check_unused_imports: true,
            check_naming_conventions: true,
            check_complexity: true,
            formatter: "fmt".to_string(),
            auto_format: false,
            fail_on_error: false,
        }
    }
}

#[derive(Debug)]
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> io::Result<Self> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| {
                io::Error::new(io::ErrorKind::NotFound, "Could not find home directory")
            })?;

        let config_dir = Path::new(&home).join(".cargo").join("fast-lint");
        fs::create_dir_all(&config_dir)?;

        Ok(Self {
            config_path: config_dir.join("config.toml"),
        })
    }

    pub fn load_config(&self) -> LintConfig {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            self.parse_config(&content).unwrap_or_default()
        } else {
            LintConfig::default()
        }
    }

    pub fn save_config(&self, config: &LintConfig) -> io::Result<()> {
        let toml_content = self.config_to_toml(config);
        fs::write(&self.config_path, toml_content)
    }

    fn parse_config(&self, content: &str) -> Option<LintConfig> {
        let mut config = LintConfig::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                if key == "max_line_length" {
                    if let Ok(val) = value.parse() {
                        config.max_line_length = val;
                    }
                } else if key == "max_nesting_depth" {
                    if let Ok(val) = value.parse() {
                        config.max_nesting_depth = val;
                    }
                } else if key == "check_todos" {
                    if let Ok(val) = value.parse() {
                        config.check_todos = val;
                    }
                } else if key == "check_trailing_whitespace" {
                    if let Ok(val) = value.parse() {
                        config.check_trailing_whitespace = val;
                    }
                } else if key == "check_missing_docs" {
                    if let Ok(val) = value.parse() {
                        config.check_missing_docs = val;
                    }
                } else if key == "check_unused_imports" {
                    if let Ok(val) = value.parse() {
                        config.check_unused_imports = val;
                    }
                } else if key == "check_naming_conventions" {
                    if let Ok(val) = value.parse() {
                        config.check_naming_conventions = val;
                    }
                } else if key == "check_complexity" {
                    if let Ok(val) = value.parse() {
                        config.check_complexity = val;
                    }
                } else if key == "formatter" {
                    config.formatter = value.to_string();
                } else if key == "auto_format" {
                    if let Ok(val) = value.parse() {
                        config.auto_format = val;
                    }
                } else if key == "fail_on_error" {
                    if let Ok(val) = value.parse() {
                        config.fail_on_error = val;
                    }
                } else {
                    eprintln!("Unknown configuration key: {}", key);
                    return None;
                }
            }
        }

        Some(config)
    }
    // The .as_str() method converts &String to &str, which is what the match arms expect.

    // Alternative solution - You can also explicitly type the variable:

    // rust
    // let key: &str = key.trim();
    // But using match key.as_str() is the most straightforward fix for this specific error.
    // Retry

    fn config_to_toml(&self, config: &LintConfig) -> String {
        format!(
            r#"# Fast-Lint Configuration
            max_line_length = {}
            max_nesting_depth = {}
            check_todos = {}
            check_trailing_whitespace = {}
            check_missing_docs = {}
            check_unused_imports = {}
            check_naming_conventions = {}
            check_complexity = {}
            formatter = "{}"
            auto_format = {}
            fail_on_error = {}
            "#,
            config.max_line_length,
            config.max_nesting_depth,
            config.check_todos,
            config.check_trailing_whitespace,
            config.check_missing_docs,
            config.check_unused_imports,
            config.check_naming_conventions,
            config.check_complexity,
            config.formatter,
            config.auto_format,
            config.fail_on_error,
        )
    }
}

pub struct FastLinter {
    config: LintConfig,
}

impl FastLinter {
    pub fn new(config: LintConfig) -> Self {
        Self { config }
    }

    pub fn run_formatter(&self, path: &Path) -> io::Result<()> {
        println!("🎨 Running formatter: cargo {}", self.config.formatter);

        let mut cmd = Command::new("cargo");
        cmd.arg(&self.config.formatter)
            .current_dir(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output()?;

        if !output.status.success() {
            eprintln!("❌ Formatter failed:");
            io::stderr().write_all(&output.stderr)?;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Formatter command failed",
            ));
        }

        if !output.stdout.is_empty() {
            io::stdout().write_all(&output.stdout)?;
        }

        println!("✅ Formatting complete");
        Ok(())
    }

    pub fn lint_project(&self, path: &Path, format_first: bool) -> io::Result<Vec<LintError>> {
        // Find Cargo.toml to ensure we're in a Rust project
        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No Cargo.toml found. Please run from a Rust project directory.",
            ));
        }

        // Run formatter if requested
        if format_first || self.config.auto_format {
            if let Err(e) = self.run_formatter(path) {
                eprintln!("⚠️  Formatter failed: {}", e);
            }
        }

        println!("🔍 Running fast-lint...");
        Ok(self.lint_directory(path))
    }

    pub fn lint_directory(&self, path: &Path) -> Vec<LintError> {
        let start = Instant::now();

        let rust_files = self.find_rust_files(path);
        println!("Found {} Rust files", rust_files.len());

        let errors = self.lint_files_parallel(&rust_files);

        let elapsed = start.elapsed();
        println!(
            "Linted {} files in {:.2}ms ({:.0} files/sec)",
            rust_files.len(),
            elapsed.as_secs_f64() * 1000.0,
            rust_files.len() as f64 / elapsed.as_secs_f64()
        );

        errors
    }

    fn find_rust_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.collect_rust_files(dir, &mut files);
        files
    }

    fn collect_rust_files(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip common directories that don't contain source code
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !matches!(name, "target" | ".git" | "node_modules" | ".cargo") {
                            self.collect_rust_files(&path, files);
                        }
                    }
                } else if path.extension().map_or(false, |ext| ext == "rs") {
                    files.push(path);
                }
            }
        }
    }

    fn lint_files_parallel(&self, files: &[PathBuf]) -> Vec<LintError> {
        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(8);

        let (tx, rx) = mpsc::channel();
        let files_per_thread = (files.len() + num_threads - 1) / num_threads;

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let start_idx = i * files_per_thread;
                let end_idx = ((i + 1) * files_per_thread).min(files.len());

                if start_idx >= files.len() {
                    return None;
                }

                let file_chunk = files[start_idx..end_idx].to_vec();
                let tx = tx.clone();
                let config = self.config.clone();

                Some(thread::spawn(move || {
                    let linter = FastLinter::new(config);
                    for file in file_chunk {
                        let errors = linter.lint_file(&file);
                        if tx.send(errors).is_err() {
                            break;
                        }
                    }
                }))
            })
            .filter_map(|h| h)
            .collect();

        drop(tx);

        let mut all_errors = Vec::new();
        while let Ok(errors) = rx.recv() {
            all_errors.extend(errors);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        all_errors
    }

    fn lint_file(&self, file_path: &Path) -> Vec<LintError> {
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(_) => return vec![],
        };

        let mut errors = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        self.check_line_issues(&lines, file_path, &mut errors);

        if self.config.check_missing_docs
            || self.config.check_unused_imports
            || self.config.check_naming_conventions
        {
            self.check_token_issues(&content, file_path, &mut errors);
        }

        if self.config.check_complexity {
            self.check_complexity_issues(&content, file_path, &mut errors);
        }

        errors
    }

    fn check_line_issues(&self, lines: &[&str], file_path: &Path, errors: &mut Vec<LintError>) {
        let mut brace_depth = 0;
        let mut in_string = false;
        let mut in_char = false;
        let mut in_line_comment = false;
        let mut in_block_comment = false;

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let mut chars = line.chars().peekable();
            let mut col = 0;

            in_line_comment = false;

            if self.config.max_line_length > 0 && line.len() > self.config.max_line_length {
                errors.push(LintError {
                    file: file_path.to_path_buf(),
                    line: line_num,
                    column: self.config.max_line_length + 1,
                    rule: "line_too_long".to_string(),
                    message: format!("Line exceeds {} characters", self.config.max_line_length),
                    severity: Severity::Warning,
                });
            }

            if self.config.check_trailing_whitespace
                && (line.ends_with(' ') || line.ends_with('\t'))
            {
                errors.push(LintError {
                    file: file_path.to_path_buf(),
                    line: line_num,
                    column: line.len(),
                    rule: "trailing_whitespace".to_string(),
                    message: "Trailing whitespace found".to_string(),
                    severity: Severity::Info,
                });
            }

            if self.config.check_todos {
                if let Some(pos) = line.find("TODO") {
                    errors.push(LintError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: pos + 1,
                        rule: "todo_found".to_string(),
                        message: "TODO comment found".to_string(),
                        severity: Severity::Info,
                    });
                }
            }

            while let Some(ch) = chars.next() {
                col += 1;

                if !in_line_comment && !in_block_comment {
                    match ch {
                        '"' if !in_char => {
                            if col == 1 || line.chars().nth(col - 2) != Some('\\') {
                                in_string = !in_string;
                            }
                        }
                        '\'' if !in_string => {
                            if col == 1 || line.chars().nth(col - 2) != Some('\\') {
                                in_char = !in_char;
                            }
                        }
                        '/' if !in_string && !in_char => {
                            if chars.peek() == Some(&'/') {
                                in_line_comment = true;
                                break;
                            } else if chars.peek() == Some(&'*') {
                                chars.next();
                                col += 1;
                                in_block_comment = true;
                            }
                        }
                        '*' if in_block_comment => {
                            if chars.peek() == Some(&'/') {
                                chars.next();
                                col += 1;
                                in_block_comment = false;
                            }
                        }
                        '{' if !in_string && !in_char && !in_line_comment && !in_block_comment => {
                            brace_depth += 1;
                            if brace_depth > self.config.max_nesting_depth {
                                errors.push(LintError {
                                    file: file_path.to_path_buf(),
                                    line: line_num,
                                    column: col,
                                    rule: "excessive_nesting".to_string(),
                                    message: format!(
                                        "Nesting depth {} exceeds maximum {}",
                                        brace_depth, self.config.max_nesting_depth
                                    ),
                                    severity: Severity::Warning,
                                });
                            }
                        }
                        '}' if !in_string && !in_char && !in_line_comment && !in_block_comment => {
                            brace_depth = brace_depth.saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn check_token_issues(&self, content: &str, file_path: &Path, errors: &mut Vec<LintError>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut imports = HashSet::new();
        let mut used_items = HashSet::new();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();

            if self.config.check_missing_docs {
                if (trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("pub struct ")
                    || trimmed.starts_with("pub enum ")
                    || trimmed.starts_with("pub trait "))
                    && line_idx > 0
                    && !lines[line_idx - 1].trim_start().starts_with("///")
                {
                    errors.push(LintError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: 1,
                        rule: "missing_docs".to_string(),
                        message: "Public item lacks documentation".to_string(),
                        severity: Severity::Warning,
                    });
                }
            }

            if self.config.check_unused_imports {
                if let Some(import) = self.extract_import(trimmed) {
                    imports.insert(import);
                }

                for import in &imports {
                    if line.contains(import) && !line.contains("use ") {
                        used_items.insert(import.clone());
                    }
                }
            }

            if self.config.check_naming_conventions {
                self.check_naming_in_line(line, line_num, file_path, errors);
            }
        }

        if self.config.check_unused_imports {
            for (line_idx, line) in lines.iter().enumerate() {
                if let Some(import) = self.extract_import(line.trim()) {
                    if !used_items.contains(&import) {
                        errors.push(LintError {
                            file: file_path.to_path_buf(),
                            line: line_idx + 1,
                            column: 1,
                            rule: "unused_import".to_string(),
                            message: format!("Unused import: {}", import),
                            severity: Severity::Warning,
                        });
                    }
                }
            }
        }
    }

    fn extract_import(&self, line: &str) -> Option<String> {
        if line.starts_with("use ") {
            if let Some(import_part) = line.strip_prefix("use ") {
                if let Some(semicolon_pos) = import_part.find(';') {
                    let import = &import_part[..semicolon_pos];
                    if let Some(last_part) = import.split("::").last() {
                        return Some(last_part.trim().to_string());
                    }
                }
            }
        }
        None
    }

    fn check_naming_in_line(
        &self,
        line: &str,
        line_num: usize,
        file_path: &Path,
        errors: &mut Vec<LintError>,
    ) {
        if let Some(fn_start) = line.find("fn ") {
            let after_fn = &line[fn_start + 3..];
            if let Some(name_end) = after_fn.find('(') {
                let fn_name = after_fn[..name_end].trim();
                if !fn_name.is_empty() && !self.is_snake_case(fn_name) {
                    errors.push(LintError {
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: fn_start + 4,
                        rule: "function_naming".to_string(),
                        message: format!("Function '{}' should be snake_case", fn_name),
                        severity: Severity::Warning,
                    });
                }
            }
        }

        if let Some(struct_start) = line.find("struct ") {
            let after_struct = &line[struct_start + 7..];
            let struct_name = after_struct.split_whitespace().next().unwrap_or("");
            if !struct_name.is_empty() && !self.is_pascal_case(struct_name) {
                errors.push(LintError {
                    file: file_path.to_path_buf(),
                    line: line_num,
                    column: struct_start + 8,
                    rule: "struct_naming".to_string(),
                    message: format!("Struct '{}' should be PascalCase", struct_name),
                    severity: Severity::Warning,
                });
            }
        }
    }

    fn is_snake_case(&self, s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
    }

    fn is_pascal_case(&self, s: &str) -> bool {
        !s.is_empty()
            && s.chars().next().unwrap().is_uppercase()
            && s.chars().all(|c| c.is_alphanumeric())
    }

    fn check_complexity_issues(
        &self,
        content: &str,
        file_path: &Path,
        errors: &mut Vec<LintError>,
    ) {
        let lines: Vec<&str> = content.lines().collect();
        let mut in_function = false;
        let mut function_start = 0;
        let mut complexity = 0;

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();

            if trimmed.contains("fn ") && trimmed.contains('{') {
                in_function = true;
                function_start = line_num;
                complexity = 1;
            }

            if in_function {
                complexity += trimmed.matches("if ").count();
                complexity += trimmed.matches("else if ").count();
                complexity += trimmed.matches("match ").count();
                complexity += trimmed.matches("for ").count();
                complexity += trimmed.matches("while ").count();
                complexity += trimmed.matches("loop ").count();
                complexity += trimmed.matches("&&").count();
                complexity += trimmed.matches("||").count();

                if trimmed.contains('}') && !trimmed.contains('{') {
                    in_function = false;
                    if complexity > 10 {
                        errors.push(LintError {
                            file: file_path.to_path_buf(),
                            line: function_start,
                            column: 1,
                            rule: "high_complexity".to_string(),
                            message: format!(
                                "Function has complexity {} (max recommended: 10)",
                                complexity
                            ),
                            severity: Severity::Warning,
                        });
                    }
                }
            }
        }
    }

    pub fn display_errors(&self, errors: &[LintError]) {
        if errors.is_empty() {
            println!("✅ No issues found!");
            return;
        }

        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;

        for error in errors {
            match error.severity {
                Severity::Error => {
                    error_count += 1;
                    print!("❌");
                }
                Severity::Warning => {
                    warning_count += 1;
                    print!("⚠️ ");
                }
                Severity::Info => {
                    info_count += 1;
                    print!("ℹ️ ");
                }
            }

            println!(
                " {}:{}:{} [{}] {}",
                error.file.display(),
                error.line,
                error.column,
                error.rule,
                error.message
            );
        }

        println!();
        println!(
            "📊 Summary: {} error(s), {} warning(s), {} info",
            error_count, warning_count, info_count
        );

        if self.config.fail_on_error && error_count > 0 {
            std::process::exit(1);
        }
    }
}
