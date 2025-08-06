use crate::rules::{Issue, Location, Rule, RuleContext, Severity, Fix, Replacement};
use syn::visit::Visit;
use syn::ExprMacro;

pub struct TodoMacroRule;

impl Rule for TodoMacroRule {
    fn name(&self) -> &'static str {
        "todo_macros"
    }

    fn check(&self, ctx: &mut RuleContext) {
        let syntax_tree = ctx.syntax_tree.clone();
        let mut visitor = TodoMacroVisitor::new(ctx);
        visitor.visit_file(&syntax_tree);
    }
}

struct TodoMacroVisitor<'a> {
    ctx: &'a mut RuleContext,
}

impl<'a> TodoMacroVisitor<'a> {
    fn new(ctx: &'a mut RuleContext) -> Self {
        Self { ctx }
    }

    fn report_todo_macro(&mut self, macro_name: &str, line: usize, col: usize, message: Option<&str>) {
        let (severity, description) = match macro_name {
            "todo" => (
                Severity::Warning,
                "TODO macro found - should be implemented before production"
            ),
            "unimplemented" => (
                Severity::Warning, 
                "Unimplemented macro found - should be implemented before production"
            ),
            "unreachable" => (
                Severity::Info,
                "Unreachable macro found - ensure this code path is truly unreachable"
            ),
            "panic" => (
                Severity::Error,
                "Panic macro found - consider returning Result or using expect() with context"
            ),
            _ => (Severity::Info, "Development macro found")
        };

        let full_message = if let Some(msg) = message {
            format!("{}: {}", description, msg)
        } else {
            description.to_string()
        };

        self.ctx.report(Issue {
            rule: "todo_macros".to_string(),
            severity,
            message: format!("{}!() - {}", macro_name, full_message),
            location: Location {
                line,
                column: col,
                end_line: Some(line),
                end_column: Some(col + macro_name.len()),
            },
            fix: match macro_name {
                "todo" => Some(Fix {
                    description: "Replace with actual implementation".to_string(),
                    replacements: vec![Replacement {
                        start: 0,
                        end: 0,
                        text: "// TODO: Implement this functionality".to_string(),
                    }],
                }),
                "unimplemented" => Some(Fix {
                    description: "Replace with actual implementation".to_string(),
                    replacements: vec![Replacement {
                        start: 0,
                        end: 0,
                        text: "return Err(\"Not yet implemented\".into())".to_string(),
                    }],
                }),
                _ => None,
            },
        });
    }
}

impl<'a> Visit<'a> for TodoMacroVisitor<'a> {
    fn visit_expr_macro(&mut self, macro_expr: &'a ExprMacro) {
        let macro_path = &macro_expr.mac.path;
        
        if let Some(last_segment) = macro_path.segments.last() {
            let macro_name = last_segment.ident.to_string();
            
            match macro_name.as_str() {
                "todo" | "unimplemented" | "unreachable" | "panic" => {
                    let (line, col) = self.ctx.line_col(last_segment.ident.span());
                    
                    // Try to extract the message from the macro
                    let message = self.extract_macro_message(&macro_expr.mac.tokens.to_string());
                    
                    self.report_todo_macro(&macro_name, line, col, message.as_deref());
                }
                _ => {}
            }
        }

        // Continue visiting
        syn::visit::visit_expr_macro(self, macro_expr);
    }
}

impl<'a> TodoMacroVisitor<'a> {
    fn extract_macro_message(&self, tokens: &str) -> Option<String> {
        let tokens = tokens.trim();
        if tokens.is_empty() {
            return None;
        }
        
        // Simple extraction - look for string literals
        if tokens.starts_with('"') && tokens.ends_with('"') {
            Some(tokens[1..tokens.len()-1].to_string())
        } else if tokens.contains('"') {
            // More complex case - extract first string literal
            let parts: Vec<&str> = tokens.split('"').collect();
            if parts.len() >= 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        } else {
            Some(tokens.to_string())
        }
    }
}