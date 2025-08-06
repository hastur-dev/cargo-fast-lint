use crate::rules::{Issue, Location, Rule, RuleContext, Severity, Fix, Replacement};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ExprCall, Pat, PatIdent, ExprMatch, Type, ExprForLoop, ExprIf, ExprLet, BinOp};
use syn::spanned::Spanned;

pub struct AntiPatternsRule;

impl Rule for AntiPatternsRule {
    fn name(&self) -> &'static str {
        "anti_patterns"
    }

    fn check(&self, ctx: &mut RuleContext) {
        let syntax_tree = ctx.syntax_tree.clone();
        let mut visitor = AntiPatternsVisitor::new(ctx);
        visitor.visit_file(&syntax_tree);
    }
}

struct AntiPatternsVisitor<'a> {
    ctx: &'a mut RuleContext,
}

impl<'a> AntiPatternsVisitor<'a> {
    fn new(ctx: &'a mut RuleContext) -> Self {
        Self { ctx }
    }

    fn report_antipattern(&mut self, line: usize, col: usize, message: &str, fix: Option<Fix>) {
        self.ctx.report(Issue {
            rule: "anti_patterns",
            severity: Severity::Warning,
            message: message.to_string(),
            location: Location {
                line,
                column: col,
                end_line: Some(line),
                end_column: Some(col + 10),
            },
            fix,
        });
    }

    fn check_unnecessary_clone(&mut self, method_call: &ExprMethodCall) {
        if method_call.method == "clone" {
            let (line, col) = self.ctx.line_col(method_call.method.span());
            
            // Check if this might be an unnecessary clone
            self.report_antipattern(
                line, 
                col,
                "Potential unnecessary clone - consider borrowing or using references",
                Some(Fix {
                    description: "Consider removing .clone() if borrowing is sufficient".to_string(),
                    replacements: vec![Replacement {
                        start: 0,
                        end: 0,
                        text: "// TODO: Review if clone is necessary".to_string(),
                    }],
                })
            );
        }
    }

    fn check_string_antipatterns(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();
        let (line, col) = self.ctx.line_col(method_call.method.span());
        
        match method_name.as_str() {
            "to_string" => {
                // Check if called on string literal
                if let Expr::Lit(lit) = method_call.receiver.as_ref() {
                    if let syn::Lit::Str(_) = &lit.lit {
                        self.report_antipattern(
                            line,
                            col,
                            "Use `String::from()` or `.to_owned()` instead of `.to_string()` on string literals",
                            Some(Fix {
                                description: "Replace with String::from()".to_string(),
                                replacements: vec![Replacement {
                                    start: 0,
                                    end: 0,
                                    text: "String::from(...)".to_string(),
                                }],
                            })
                        );
                    }
                }
            }
            "as_str" => {
                // Check for redundant as_str() calls
                if let Expr::MethodCall(inner) = method_call.receiver.as_ref() {
                    if inner.method == "to_string" {
                        self.report_antipattern(
                            line,
                            col,
                            "Redundant `.to_string().as_str()` - just use the original string",
                            None
                        );
                    }
                }
            }
            "len" => {
                // Check for .len() == 0 instead of .is_empty()
                // This would need more context analysis
            }
            _ => {}
        }
    }

    fn check_collection_antipatterns(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();
        let (line, col) = self.ctx.line_col(method_call.method.span());
        
        match method_name.as_str() {
            "collect" => {
                // Check for collect followed by indexing
                self.ctx.report(Issue {
                    rule: "anti_patterns",
                    severity: Severity::Info,
                    message: "Consider if iteration can be done without collecting - lazy evaluation is often more efficient".to_string(),
                    location: Location {
                        line,
                        column: col,
                        end_line: Some(line),
                        end_column: Some(col + method_name.len()),
                    },
                    fix: None,
                });
            }
            "into_iter" => {
                // Check if called on reference
                if let Expr::Reference(_) = method_call.receiver.as_ref() {
                    self.report_antipattern(
                        line,
                        col,
                        "Use `.iter()` instead of `(&collection).into_iter()`",
                        Some(Fix {
                            description: "Replace with .iter()".to_string(),
                            replacements: vec![Replacement {
                                start: 0,
                                end: 0,
                                text: ".iter()".to_string(),
                            }],
                        })
                    );
                }
            }
            _ => {}
        }
    }

    fn check_option_result_patterns(&mut self, method_call: &ExprMethodCall) {
        let method_name = method_call.method.to_string();
        let (line, col) = self.ctx.line_col(method_call.method.span());

        match method_name.as_str() {
            "is_some" | "is_none" | "is_ok" | "is_err" => {
                // These are often followed by unwrap/expect - common antipattern
                self.ctx.report(Issue {
                    rule: "anti_patterns",
                    severity: Severity::Info,
                    message: format!("Consider using pattern matching or combinators instead of `.{}()` checks", method_name),
                    location: Location {
                        line,
                        column: col,
                        end_line: Some(line),
                        end_column: Some(col + method_name.len()),
                    },
                    fix: None,
                });
            }
            "map" => {
                // Check for map(|x| x) - identity map
                if let Some(_closure) = method_call.args.first() {
                    // This would need more sophisticated analysis
                }
            }
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for AntiPatternsVisitor<'a> {
    fn visit_expr_method_call(&mut self, method_call: &'a ExprMethodCall) {
        self.check_unnecessary_clone(method_call);
        self.check_string_antipatterns(method_call);
        self.check_collection_antipatterns(method_call);
        self.check_option_result_patterns(method_call);

        // Check for specific problematic patterns
        let method_name = method_call.method.to_string();
        match method_name.as_str() {
            "get" => {
                // Check for vec.get(0) instead of vec.first()
                if let Some(first_arg) = method_call.args.first() {
                    if let Expr::Lit(lit) = first_arg {
                        if let syn::Lit::Int(int_lit) = &lit.lit {
                            if int_lit.base10_digits() == "0" {
                                let (line, col) = self.ctx.line_col(method_call.method.span());
                                self.report_antipattern(
                                    line,
                                    col,
                                    "Use `.first()` instead of `.get(0)` for better semantics",
                                    Some(Fix {
                                        description: "Replace with .first()".to_string(),
                                        replacements: vec![Replacement {
                                            start: 0,
                                            end: 0,
                                            text: ".first()".to_string(),
                                        }],
                                    })
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Continue visiting
        syn::visit::visit_expr_method_call(self, method_call);
    }

    fn visit_expr_if(&mut self, if_expr: &'a syn::ExprIf) {
        // Check for if let Some(_) = ... { true } else { false } patterns
        if let Expr::Let(let_expr) = if_expr.cond.as_ref() {
            if let Some(else_branch) = &if_expr.else_branch {
                // Check if this is a boolean conversion pattern
                let (line, col) = self.ctx.line_col(let_expr.let_token.span());
                self.ctx.report(Issue {
                    rule: "anti_patterns",
                    severity: Severity::Info,
                    message: "Consider using `.is_some()`, `.is_none()`, `.is_ok()`, or `.is_err()` instead of if-let for boolean conversion".to_string(),
                    location: Location {
                        line,
                        column: col,
                        end_line: Some(line),
                        end_column: Some(col + 5),
                    },
                    fix: None,
                });
            }
        }

        // Continue visiting
        syn::visit::visit_expr_if(self, if_expr);
    }

    fn visit_expr_match(&mut self, match_expr: &'a ExprMatch) {
        // Check for match expressions that could be simplified
        if match_expr.arms.len() == 2 {
            let (line, col) = self.ctx.line_col(match_expr.match_token.span());
            
            // Look for Ok/Err or Some/None patterns that could use combinators
            let mut has_option_result_pattern = false;
            
            for arm in &match_expr.arms {
                if let Pat::Path(path) = &arm.pat {
                    if let Some(last_segment) = path.path.segments.last() {
                        let name = last_segment.ident.to_string();
                        if matches!(name.as_str(), "Some" | "None" | "Ok" | "Err") {
                            has_option_result_pattern = true;
                            break;
                        }
                    }
                }
            }

            if has_option_result_pattern {
                self.ctx.report(Issue {
                    rule: "anti_patterns",
                    severity: Severity::Info,
                    message: "Consider using combinators like `.map()`, `.and_then()`, `.unwrap_or()`, etc. instead of match for simple Option/Result handling".to_string(),
                    location: Location {
                        line,
                        column: col,
                        end_line: Some(line),
                        end_column: Some(col + 5),
                    },
                    fix: None,
                });
            }
        }

        // Continue visiting
        syn::visit::visit_expr_match(self, match_expr);
    }

    fn visit_expr_for_loop(&mut self, for_loop: &'a ExprForLoop) {
        // Check for for loops that could be replaced with iterators
        if let Expr::Range(range) = for_loop.expr.as_ref() {
            if let (Some(start), Some(end)) = (&range.start, &range.end) {
                let (line, col) = self.ctx.line_col(for_loop.for_token.span());
                self.ctx.report(Issue {
                    rule: "anti_patterns",
                    severity: Severity::Info,
                    message: "Consider using iterator methods like `.enumerate()`, `.zip()`, or range methods instead of indexed for loops".to_string(),
                    location: Location {
                        line,
                        column: col,
                        end_line: Some(line),
                        end_column: Some(col + 3),
                    },
                    fix: None,
                });
            }
        }

        // Continue visiting
        syn::visit::visit_expr_for_loop(self, for_loop);
    }
}