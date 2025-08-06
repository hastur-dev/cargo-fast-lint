use super::*;

pub struct UnsafeBlockRule;

impl Rule for UnsafeBlockRule {
    fn name(&self) -> &'static str {
        "unsafe-block"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        let mut issues_to_report = Vec::new();
        
        // Check unsafe functions first
        for item in &ctx.syntax_tree.items {
            if let syn::Item::Fn(func) = item {
                if func.sig.unsafety.is_some() {
                    let (line, col) = ctx.line_col(func.sig.ident.span());
                    
                    // Check for safety documentation
                    let has_safety_doc = func.attrs.iter().any(|attr| {
                        if attr.path().is_ident("doc") {
                            // For now, just assume it doesn't have safety docs
                            // A proper implementation would parse the attribute value
                            false
                        } else {
                            false
                        }
                    });
                    
                    if !has_safety_doc {
                        issues_to_report.push(Issue {
                            rule: self.name(),
                            severity: Severity::Error,
                            message: "Unsafe function without safety documentation".to_string(),
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
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
        
        // TODO: Add unsafe block checking by traversing the AST more carefully
        // For now, we'll skip the complex visitor pattern that causes borrowing issues
    }
}