use super::*;

pub struct NamingConventionRule;

impl Rule for NamingConventionRule {
    fn name(&self) -> &'static str {
        "naming-convention"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        // Collect issues first to avoid borrowing conflicts
        let mut issues_to_report = Vec::new();
        
        for item in &ctx.syntax_tree.items {
            match item {
                syn::Item::Fn(func) => {
                    let name = func.sig.ident.to_string();
                    if !is_snake_case(&name) && !name.starts_with("test_") {
                        let (line, col) = ctx.line_col(func.sig.ident.span());
                        issues_to_report.push(Issue {
                            rule: self.name(),
                            severity: Severity::Warning,
                            message: format!("Function '{}' should be snake_case", name),
                            location: Location {
                                line,
                                column: col,
                                end_line: None,
                                end_column: None,
                            },
                            fix: Some(Fix {
                                description: "Convert to snake_case".to_string(),
                                replacements: vec![Replacement {
                                    start: 0, // Would calculate actual position
                                    end: 0,
                                    text: to_snake_case(&name),
                                }],
                            }),
                        });
                    }
                }
                syn::Item::Struct(s) => {
                    let name = s.ident.to_string();
                    if !is_pascal_case(&name) {
                        let (line, col) = ctx.line_col(s.ident.span());
                        issues_to_report.push(Issue {
                            rule: self.name(),
                            severity: Severity::Warning,
                            message: format!("Struct '{}' should be PascalCase", name),
                            location: Location {
                                line,
                                column: col,
                                end_line: None,
                                end_column: None,
                            },
                            fix: None,
                        });
                    }
                }
                _ => {}
            }
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
    }
}

pub struct LineLengthRule {
    max_length: usize,
}

impl LineLengthRule {
    pub fn new(max_length: usize) -> Self {
        Self { max_length }
    }
}

impl Rule for LineLengthRule {
    fn name(&self) -> &'static str {
        "line-too-long"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        let lines: Vec<_> = ctx.content.lines().enumerate().collect();
        let mut issues_to_report = Vec::new();
        
        for (i, line) in lines {
            if line.len() > self.max_length {
                issues_to_report.push(Issue {
                    rule: self.name(),
                    severity: Severity::Info,
                    message: format!(
                        "Line exceeds {} characters ({})",
                        self.max_length,
                        line.len()
                    ),
                    location: Location {
                        line: i + 1,
                        column: self.max_length + 1,
                        end_line: None,
                        end_column: None,
                    },
                    fix: None,
                });
            }
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
    }
}

// Helper functions
fn is_snake_case(s: &str) -> bool {
    s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
}

fn is_pascal_case(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_uppercase())
        && s.chars().all(|c| c.is_alphanumeric())
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}