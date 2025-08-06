use super::*;

pub struct UnmatchedDelimitersRule;

impl Rule for UnmatchedDelimitersRule {
    fn name(&self) -> &'static str {
        "unmatched-delimiters"
    }
    
    fn check(&self, ctx: &mut RuleContext) {
        // This is handled by syn parsing - if we got here, delimiters match
        // But we can check for common issues in raw strings
        
        let mut depth = 0;
        let in_string = false;
        let in_comment = false;
        let mut issues_to_report = Vec::new();
        
        let lines: Vec<_> = ctx.content.lines().enumerate().collect();
        for (i, line) in lines {
            let mut chars = line.chars().peekable();
            let mut col = 0;
            
            while let Some(ch) = chars.next() {
                col += 1;
                
                match ch {
                    '{' if !in_string && !in_comment => depth += 1,
                    '}' if !in_string && !in_comment => {
                        depth -= 1;
                        if depth < 0 {
                            issues_to_report.push(Issue {
                                rule: self.name().to_string(),
                                severity: Severity::Error,
                                message: "Unmatched closing brace".to_string(),
                                location: Location {
                                    line: i + 1,
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
        }
        
        // Report all issues
        for issue in issues_to_report {
            ctx.report(issue);
        }
    }
}

pub struct InvalidSyntaxRule;

impl Rule for InvalidSyntaxRule {
    fn name(&self) -> &'static str {
        "invalid-syntax"
    }
    
    fn check(&self, _ctx: &mut RuleContext) {
        // Check for common syntax issues that syn might accept but are problematic
        // For now, this is a simplified implementation that doesn't detect any issues
        // A real implementation would traverse the AST and check for specific patterns
        
        // Since syn successfully parsed the file, there are no syntax errors
        // This rule would be more useful for detecting deprecated syntax or
        // patterns that compile but are considered problematic
    }
}