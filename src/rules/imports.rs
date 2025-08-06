use super::*;
use std::collections::{HashSet, HashMap};
use syn::spanned::Spanned;

pub struct ImportOrderRule;

impl Rule for ImportOrderRule {
    fn name(&self) -> &'static str {
        "import-order"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        let mut std_imports = vec![];
        let mut external_imports = vec![];
        let mut local_imports = vec![];
        let mut issues_to_report = Vec::new();
        
        for item in &ctx.syntax_tree.items {
            if let syn::Item::Use(use_item) = item {
                let path = use_path_to_string(&use_item.tree);
                let (line, col) = ctx.line_col(use_item.span());
                
                if path.starts_with("std::") || path.starts_with("core::") || path.starts_with("alloc::") {
                    std_imports.push((path, line, col));
                } else if path.starts_with("crate::") || path.starts_with("super::") || path.starts_with("self::") {
                    local_imports.push((path, line, col));
                } else {
                    external_imports.push((path, line, col));
                }
            }
        }
        
        // Check if imports are grouped correctly
        let mut last_std_line = 0;
        let mut last_external_line = 0;
        
        for (_, line, _) in &std_imports {
            last_std_line = last_std_line.max(*line);
        }
        
        for (_, line, col) in &external_imports {
            if *line < last_std_line {
                issues_to_report.push(Issue {
                    rule: self.name(),
                    severity: Severity::Info,
                    message: "External imports should come after standard library imports".to_string(),
                    location: Location {
                        line: *line,
                        column: *col,
                        end_line: None,
                        end_column: None,
                    },
                    fix: None,
                });
            }
            last_external_line = last_external_line.max(*line);
        }
        
        for (_, line, col) in &local_imports {
            if *line < last_external_line {
                issues_to_report.push(Issue {
                    rule: self.name(),
                    severity: Severity::Info,
                    message: "Local imports should come after external crate imports".to_string(),
                    location: Location {
                        line: *line,
                        column: *col,
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

pub struct UnusedImportRule;

impl Rule for UnusedImportRule {
    fn name(&self) -> &'static str {
        "unused-import"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        let mut imports = HashMap::new();
        let mut used_idents = HashSet::new();
        let mut issues_to_report = Vec::new();
        
        // Collect all imports
        for item in &ctx.syntax_tree.items {
            if let syn::Item::Use(use_item) = item {
                collect_use_tree_idents(&use_item.tree, &mut imports, ctx);
            }
        }
        
        // Collect all used identifiers
        struct IdentCollector<'a> {
            used: &'a mut HashSet<String>,
        }
        
        impl<'ast> Visit<'ast> for IdentCollector<'_> {
            fn visit_ident(&mut self, ident: &'ast syn::Ident) {
                self.used.insert(ident.to_string());
            }
        }
        
        let mut collector = IdentCollector { used: &mut used_idents };
        for item in &ctx.syntax_tree.items {
            // Skip the use statements themselves
            if !matches!(item, syn::Item::Use(_)) {
                collector.visit_item(item);
            }
        }
        
        // Report unused imports
        for (name, (line, col)) in imports {
            if !used_idents.contains(&name) {
                issues_to_report.push(Issue {
                    rule: self.name(),
                    severity: Severity::Warning,
                    message: format!("Unused import: {}", name),
                    location: Location {
                        line,
                        column: col,
                        end_line: None,
                        end_column: None,
                    },
                    fix: Some(Fix {
                        description: "Remove unused import".to_string(),
                        replacements: vec![], // Would calculate actual removal
                    }),
                });
            }
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
    }
}

fn use_path_to_string(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => format!("{}::{}", p.ident, use_path_to_string(&p.tree)),
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(g) => {
            let items: Vec<_> = g.items.iter().map(use_path_to_string).collect();
            format!("{{{}}}", items.join(", "))
        }
        syn::UseTree::Rename(r) => format!("{} as {}", r.ident, r.rename),
    }
}

fn collect_use_tree_idents(
    tree: &syn::UseTree,
    imports: &mut HashMap<String, (usize, usize)>,
    ctx: &RuleContext,
) {
    match tree {
        syn::UseTree::Name(n) => {
            let (line, col) = ctx.line_col(n.ident.span());
            imports.insert(n.ident.to_string(), (line, col));
        }
        syn::UseTree::Rename(r) => {
            let (line, col) = ctx.line_col(r.rename.span());
            imports.insert(r.rename.to_string(), (line, col));
        }
        syn::UseTree::Path(p) => {
            collect_use_tree_idents(&p.tree, imports, ctx);
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                collect_use_tree_idents(item, imports, ctx);
            }
        }
        _ => {}
    }
}