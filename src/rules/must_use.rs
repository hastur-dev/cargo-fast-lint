use crate::rules::{Issue, Location, Rule, RuleContext, Severity};
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, Item, ItemFn, ReturnType, Attribute, Meta};

pub struct MustUseRule;

impl Rule for MustUseRule {
    fn name(&self) -> &'static str {
        "must_use_violations"
    }

    fn check(&self, ctx: &mut RuleContext) {
        let syntax_tree = ctx.syntax_tree.clone();
        let mut visitor = MustUseVisitor::new(ctx);
        visitor.visit_file(&syntax_tree);
    }
}

struct MustUseVisitor<'a> {
    ctx: &'a mut RuleContext,
    must_use_functions: Vec<String>,
}

impl<'a> MustUseVisitor<'a> {
    fn new(ctx: &'a mut RuleContext) -> Self {
        Self {
            ctx,
            must_use_functions: vec![
                // Common std functions that return must_use types
                "collect".to_string(),
                "map".to_string(),
                "filter".to_string(),
                "fold".to_string(),
                "reduce".to_string(),
                "try_fold".to_string(),
                "try_reduce".to_string(),
                "cloned".to_string(),
                "copied".to_string(),
                "enumerate".to_string(),
                "zip".to_string(),
                "chain".to_string(),
                "take".to_string(),
                "skip".to_string(),
                "rev".to_string(),
            ],
        }
    }

    fn has_must_use_attr(&self, attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| {
            if let Meta::Path(path) = &attr.meta {
                path.is_ident("must_use")
            } else if let Meta::List(list) = &attr.meta {
                list.path.is_ident("must_use")
            } else {
                false
            }
        })
    }

    fn check_unused_result(&mut self, _expr: &Expr, line: usize, col: usize, context: &str) {
        self.ctx.report(Issue {
            rule: "must_use_violations",
            severity: Severity::Warning,
            message: format!("Unused result from {} - consider using `let _ = ...` if intentional", context),
            location: Location {
                line,
                column: col,
                end_line: Some(line),
                end_column: Some(col + 10), // Approximate
            },
            fix: None,
        });
    }

    fn is_result_ignored(&self, parent_expr: Option<&Expr>) -> bool {
        // Check if this is a standalone statement (not assigned or used)
        match parent_expr {
            None => true, // Top-level expression
            Some(Expr::Block(_)) => true, // Statement in block
            Some(Expr::If(_)) => false, // Used in condition
            Some(Expr::Match(_)) => false, // Used in match
            Some(Expr::Let(_)) => false, // Assigned to variable
            Some(Expr::Assign(_)) => false, // Part of assignment
            _ => true, // Default to checking
        }
    }
}

impl<'a> Visit<'a> for MustUseVisitor<'a> {
    fn visit_item_fn(&mut self, item_fn: &'a ItemFn) {
        // Check if function has #[must_use] and track it
        if self.has_must_use_attr(&item_fn.attrs) {
            let fn_name = item_fn.sig.ident.to_string();
            self.must_use_functions.push(fn_name);
        }

        // Continue visiting the function body
        syn::visit::visit_item_fn(self, item_fn);
    }

    fn visit_expr_call(&mut self, call: &'a ExprCall) {
        // Check for calls to functions that return must_use types
        if let Expr::Path(path) = call.func.as_ref() {
            if let Some(last_segment) = path.path.segments.last() {
                let func_name = last_segment.ident.to_string();
                
                if self.must_use_functions.contains(&func_name) {
                    let (line, col) = self.ctx.line_col(last_segment.ident.span());
                    self.check_unused_result(
                        &Expr::Call(call.clone()),
                        line,
                        col,
                        &format!("function call `{}`", func_name)
                    );
                }
                
                // Check for specific patterns
                match func_name.as_str() {
                    "write" | "writeln" | "print" | "println" => {
                        // These are commonly ignored, but should be checked
                        let (line, col) = self.ctx.line_col(last_segment.ident.span());
                        self.ctx.report(Issue {
                            rule: "must_use_violations",
                            severity: Severity::Info,
                            message: format!("Consider checking the result of `{}()` for error handling", func_name),
                            location: Location {
                                line,
                                column: col,
                                end_line: Some(line),
                                end_column: Some(col + func_name.len()),
                            },
                            fix: None,
                        });
                    }
                    _ => {}
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, method_call: &'a ExprMethodCall) {
        let method_name = method_call.method.to_string();
        
        // Check for iterator methods that should be consumed
        if self.must_use_functions.contains(&method_name) {
            let (line, col) = self.ctx.line_col(method_call.method.span());
            
            match method_name.as_str() {
                "map" | "filter" | "enumerate" | "zip" | "chain" | "take" | "skip" | "rev" => {
                    self.ctx.report(Issue {
                        rule: "must_use_violations",
                        severity: Severity::Warning,
                        message: format!("Iterator method `{}()` returns a lazy iterator that must be consumed (e.g., with `.collect()`, `.for_each()`, etc.)", method_name),
                        location: Location {
                            line,
                            column: col,
                            end_line: Some(line),
                            end_column: Some(col + method_name.len()),
                        },
                        fix: None,
                    });
                }
                "collect" => {
                    // This is good - consuming the iterator
                }
                _ => {
                    self.check_unused_result(
                        &Expr::MethodCall(method_call.clone()),
                        line,
                        col,
                        &format!("method call `.{}`", method_name)
                    );
                }
            }
        }

        // Check for Result/Option methods
        match method_name.as_str() {
            "ok" | "err" | "unwrap_or" | "unwrap_or_else" | "unwrap_or_default" => {
                // These convert Result/Option and should often be used
                let (line, col) = self.ctx.line_col(method_call.method.span());
                self.ctx.report(Issue {
                    rule: "must_use_violations",
                    severity: Severity::Info,
                    message: format!("Result of `.{}()` should typically be used or explicitly ignored", method_name),
                    location: Location {
                        line,
                        column: col,
                        end_line: Some(line),
                        end_column: Some(col + method_name.len()),
                    },
                    fix: None,
                });
            }
            _ => {}
        }

        // Continue visiting
        syn::visit::visit_expr_method_call(self, method_call);
    }
}