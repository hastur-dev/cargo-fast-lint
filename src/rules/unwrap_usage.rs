use crate::rules::{Issue, Location, Rule, RuleContext, Severity, Fix, Replacement};
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall};

pub struct UnwrapUsageRule;

impl Rule for UnwrapUsageRule {
    fn name(&self) -> &'static str {
        "unwrap_usage"
    }

    fn check(&self, ctx: &mut RuleContext) {
        let syntax_tree = ctx.syntax_tree.clone();
        let mut visitor = UnwrapVisitor::new(ctx);
        visitor.visit_file(&syntax_tree);
    }
}

struct UnwrapVisitor<'a> {
    ctx: &'a mut RuleContext,
}

impl<'a> UnwrapVisitor<'a> {
    fn new(ctx: &'a mut RuleContext) -> Self {
        Self { ctx }
    }

    fn report_unwrap(&mut self, method_name: &str, line: usize, col: usize) {
        let suggestion = match method_name {
            "unwrap" => "Consider using `match`, `if let`, or `expect()` with a descriptive message",
            "unwrap_or_default" => "This is generally safe, but consider explicit handling",
            "expect" => "Good! Using expect() with descriptive messages",
            _ => "Consider explicit error handling"
        };

        let severity = match method_name {
            "unwrap" => Severity::Warning,
            "unwrap_unchecked" => Severity::Error,
            _ => Severity::Info,
        };

        self.ctx.report(Issue {
            rule: "unwrap_usage".to_string(),
            severity,
            message: format!("Found `{}()` call - {}", method_name, suggestion),
            location: Location {
                line,
                column: col,
                end_line: Some(line),
                end_column: Some(col + method_name.len()),
            },
            fix: if method_name == "unwrap" {
                Some(Fix {
                    description: format!("Replace with expect() and descriptive message"),
                    replacements: vec![Replacement {
                        start: 0, // This would need proper span calculation
                        end: 0,
                        text: "expect(\"TODO: Add descriptive error message\")".to_string(),
                    }],
                })
            } else {
                None
            },
        });
    }
}

impl<'a> Visit<'a> for UnwrapVisitor<'a> {
    fn visit_expr_method_call(&mut self, method_call: &'a ExprMethodCall) {
        let method_name = method_call.method.to_string();
        
        match method_name.as_str() {
            "unwrap" | "unwrap_or_default" | "unwrap_unchecked" | "expect" => {
                let (line, col) = self.ctx.line_col(method_call.method.span());
                self.report_unwrap(&method_name, line, col);
            }
            _ => {}
        }

        // Continue visiting
        syn::visit::visit_expr_method_call(self, method_call);
    }

    fn visit_expr_call(&mut self, call: &'a ExprCall) {
        // Check for unwrap-like function calls
        if let Expr::Path(path) = call.func.as_ref() {
            if let Some(last_segment) = path.path.segments.last() {
                let func_name = last_segment.ident.to_string();
                if func_name.contains("unwrap") {
                    let (line, col) = self.ctx.line_col(last_segment.ident.span());
                    self.report_unwrap(&func_name, line, col);
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_call(self, call);
    }
}