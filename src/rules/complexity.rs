use super::*;

pub struct CyclomaticComplexityRule {
    max_complexity: usize,
}

impl CyclomaticComplexityRule {
    pub fn new(max_complexity: usize) -> Self {
        Self { max_complexity }
    }
}

impl Rule for CyclomaticComplexityRule {
    fn name(&self) -> &'static str {
        "cyclomatic-complexity"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        let mut issues_to_report = Vec::new();
        
        // Process functions directly from ctx to get proper line info
        for item in &ctx.syntax_tree.items {
            if let syn::Item::Fn(func) = item {
                let complexity = calculate_cyclomatic_complexity(&func.block.stmts);
                
                if complexity > self.max_complexity {
                    let (line, col) = ctx.line_col(func.sig.ident.span());
                    issues_to_report.push(Issue {
                        rule: self.name(),
                        severity: Severity::Warning,
                        message: format!(
                            "Function '{}' has cyclomatic complexity of {} (max: {})",
                            func.sig.ident,
                            complexity,
                            self.max_complexity
                        ),
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
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
    }
}

pub struct CognitiveComplexityRule {
    max_complexity: usize,
}

impl CognitiveComplexityRule {
    pub fn new(max_complexity: usize) -> Self {
        Self { max_complexity }
    }
}

impl Rule for CognitiveComplexityRule {
    fn name(&self) -> &'static str {
        "cognitive-complexity"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        let mut issues_to_report = Vec::new();
        
        for item in &ctx.syntax_tree.items {
            if let syn::Item::Fn(func) = item {
                let complexity = calculate_cognitive_complexity(&func.block.stmts, 0);
                
                if complexity > self.max_complexity {
                    let (line, col) = ctx.line_col(func.sig.ident.span());
                    issues_to_report.push(Issue {
                        rule: self.name(),
                        severity: Severity::Warning,
                        message: format!(
                            "Function '{}' has cognitive complexity of {} (max: {})",
                            func.sig.ident,
                            complexity,
                            self.max_complexity
                        ),
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
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
    }
}

fn calculate_cyclomatic_complexity(stmts: &[syn::Stmt]) -> usize {
    let mut complexity = 1; // Base complexity
    
    for stmt in stmts {
        complexity += count_decision_points_stmt(stmt);
    }
    
    complexity
}

fn calculate_cognitive_complexity(stmts: &[syn::Stmt], nesting_level: usize) -> usize {
    let mut complexity = 0;
    
    for stmt in stmts {
        complexity += count_cognitive_complexity_stmt(stmt, nesting_level);
    }
    
    complexity
}

fn count_decision_points_stmt(stmt: &syn::Stmt) -> usize {
    match stmt {
        syn::Stmt::Expr(expr, _) => count_decision_points_expr(expr),
        syn::Stmt::Local(local) => {
            local.init.as_ref()
                .map(|init| count_decision_points_expr(&init.expr))
                .unwrap_or(0)
        }
        _ => 0,
    }
}

fn count_decision_points_expr(expr: &syn::Expr) -> usize {
    match expr {
        syn::Expr::If(_) => 1,
        syn::Expr::Match(m) => m.arms.len().saturating_sub(1), // n-1 for match arms
        syn::Expr::While(_) | syn::Expr::ForLoop(_) | syn::Expr::Loop(_) => 1,
        syn::Expr::Binary(bin) => {
            match bin.op {
                syn::BinOp::And(_) | syn::BinOp::Or(_) => 1,
                _ => 0,
            }
        }
        // Recursively check sub-expressions
        _ => 0, // Simplified for now
    }
}

fn count_cognitive_complexity_stmt(stmt: &syn::Stmt, nesting_level: usize) -> usize {
    match stmt {
        syn::Stmt::Expr(expr, _) => {
            count_cognitive_complexity_expr(expr, nesting_level)
        }
        syn::Stmt::Local(local) => {
            local.init.as_ref()
                .map(|init| count_cognitive_complexity_expr(&init.expr, nesting_level))
                .unwrap_or(0)
        }
        _ => 0,
    }
}

fn count_cognitive_complexity_expr(expr: &syn::Expr, nesting_level: usize) -> usize {
    match expr {
        syn::Expr::If(_) => 1 + nesting_level,
        syn::Expr::Match(_) => 1 + nesting_level,
        syn::Expr::While(_) | syn::Expr::ForLoop(_) | syn::Expr::Loop(_) => 1 + nesting_level,
        syn::Expr::Binary(bin) => {
            match bin.op {
                syn::BinOp::And(_) | syn::BinOp::Or(_) => 1,
                _ => 0,
            }
        }
        // Recursively check sub-expressions with increased nesting
        _ => 0, // Simplified for now
    }
}