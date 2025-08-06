use super::*;

pub struct MissingDocsRule;

impl Rule for MissingDocsRule {
    fn name(&self) -> &'static str {
        "missing-docs"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        // Collect items to check first to avoid borrowing issues
        let mut issues_to_report = Vec::new();
        
        for item in &ctx.syntax_tree.items {
            match item {
                syn::Item::Fn(f) if is_pub(&f.vis) => {
                    if !has_doc_comment(&f.attrs) {
                        let (line, col) = ctx.line_col(f.sig.ident.span());
                        issues_to_report.push(Issue {
                            rule: self.name().to_string(),
                            severity: Severity::Warning,
                            message: format!("Missing documentation for public function '{}'", f.sig.ident),
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
                syn::Item::Struct(s) if is_pub(&s.vis) => {
                    if !has_doc_comment(&s.attrs) {
                        let (line, col) = ctx.line_col(s.ident.span());
                        issues_to_report.push(Issue {
                            rule: self.name().to_string(),
                            severity: Severity::Warning,
                            message: format!("Missing documentation for public struct '{}'", s.ident),
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
                syn::Item::Enum(e) if is_pub(&e.vis) => {
                    if !has_doc_comment(&e.attrs) {
                        let (line, col) = ctx.line_col(e.ident.span());
                        issues_to_report.push(Issue {
                            rule: self.name().to_string(),
                            severity: Severity::Warning,
                            message: format!("Missing documentation for public enum '{}'", e.ident),
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
                syn::Item::Trait(t) if is_pub(&t.vis) => {
                    if !has_doc_comment(&t.attrs) {
                        let (line, col) = ctx.line_col(t.ident.span());
                        issues_to_report.push(Issue {
                            rule: self.name().to_string(),
                            severity: Severity::Warning,
                            message: format!("Missing documentation for public trait '{}'", t.ident),
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

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn has_doc_comment(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("doc"))
}