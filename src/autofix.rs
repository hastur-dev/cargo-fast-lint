use crate::rules::{Fix, Issue, Location, Replacement};
use std::path::Path;
use syn::{File as SynFile, Item, ItemUse, UseTree};

pub struct AutoFixEngine {
    pub fixes_applied: usize,
}

impl AutoFixEngine {
    pub fn new() -> Self {
        Self {
            fixes_applied: 0,
        }
    }
    
    pub fn apply_fixes(&mut self, content: &str, issues: &[Issue]) -> Result<String, Box<dyn std::error::Error>> {
        let mut fixed_content = content.to_string();
        let mut offset_adjustment = 0i64;
        
        // Sort fixes by position (reverse order to maintain positions)
        let mut fixes_with_positions: Vec<_> = issues
            .iter()
            .filter_map(|issue| issue.fix.as_ref().map(|fix| (issue, fix)))
            .collect();
        
        // Sort by start position in reverse order
        fixes_with_positions.sort_by_key(|(issue, _)| std::cmp::Reverse(issue.location.line));
        
        for (issue, fix) in fixes_with_positions {
            match self.apply_single_fix(&mut fixed_content, issue, fix, &mut offset_adjustment) {
                Ok(true) => self.fixes_applied += 1,
                Ok(false) => {}, // Fix not applicable
                Err(e) => eprintln!("Warning: Failed to apply fix for {}: {}", issue.rule, e),
            }
        }
        
        Ok(fixed_content)
    }
    
    fn apply_single_fix(
        &self,
        content: &mut String,
        issue: &Issue,
        fix: &Fix,
        offset_adjustment: &mut i64,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        for replacement in &fix.replacements {
            let start = (replacement.start as i64 + *offset_adjustment) as usize;
            let end = (replacement.end as i64 + *offset_adjustment) as usize;
            
            if start > content.len() || end > content.len() || start > end {
                return Ok(false); // Invalid range, skip this fix
            }
            
            let original_len = end - start;
            let new_len = replacement.text.len();
            
            content.replace_range(start..end, &replacement.text);
            
            // Update offset for subsequent fixes
            *offset_adjustment += new_len as i64 - original_len as i64;
        }
        
        Ok(true)
    }
}

// Import reorganization functionality
pub struct ImportOrganizer {
    pub preserve_comments: bool,
    pub group_external_crates: bool,
    pub sort_within_groups: bool,
}

impl ImportOrganizer {
    pub fn new() -> Self {
        Self {
            preserve_comments: true,
            group_external_crates: true,
            sort_within_groups: true,
        }
    }
    
    pub fn organize_imports(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let syntax_tree: SynFile = syn::parse_str(content)?;
        
        let mut use_items = Vec::new();
        let mut non_use_items = Vec::new();
        let mut use_positions = Vec::new();
        
        for (index, item) in syntax_tree.items.iter().enumerate() {
            if let Item::Use(use_item) = item {
                use_items.push(use_item.clone());
                use_positions.push(index);
            } else {
                non_use_items.push(item.clone());
            }
        }
        
        if use_items.is_empty() {
            return Ok(content.to_string());
        }
        
        // Organize imports into groups
        let organized_imports = self.group_and_sort_imports(use_items)?;
        
        // Reconstruct the file with organized imports
        self.reconstruct_file_with_organized_imports(content, &organized_imports, &use_positions)
    }
    
    fn group_and_sort_imports(&self, use_items: Vec<ItemUse>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut std_imports = Vec::new();
        let mut external_imports = Vec::new();
        let mut local_imports = Vec::new();
        
        for use_item in use_items {
            let import_str = quote::quote!(#use_item).to_string();
            let cleaned = import_str.replace(" ", "").replace("\n", "");
            
            if self.is_std_import(&use_item) {
                std_imports.push(import_str);
            } else if self.is_external_import(&use_item) {
                external_imports.push(import_str);
            } else {
                local_imports.push(import_str);
            }
        }
        
        if self.sort_within_groups {
            std_imports.sort();
            external_imports.sort();
            local_imports.sort();
        }
        
        let mut organized = Vec::new();
        
        if !std_imports.is_empty() {
            organized.extend(std_imports);
            organized.push(String::new()); // Empty line separator
        }
        
        if !external_imports.is_empty() {
            organized.extend(external_imports);
            organized.push(String::new()); // Empty line separator
        }
        
        if !local_imports.is_empty() {
            organized.extend(local_imports);
        }
        
        // Remove trailing empty line
        if organized.last() == Some(&String::new()) {
            organized.pop();
        }
        
        Ok(organized)
    }
    
    fn is_std_import(&self, use_item: &ItemUse) -> bool {
        let path_str = self.use_tree_to_string(&use_item.tree);
        path_str.starts_with("std::") || 
        path_str.starts_with("core::") || 
        path_str.starts_with("alloc::")
    }
    
    fn is_external_import(&self, use_item: &ItemUse) -> bool {
        let path_str = self.use_tree_to_string(&use_item.tree);
        // Simple heuristic: if it doesn't start with std/core/alloc and doesn't contain "::" in the first segment,
        // it's likely an external crate
        if self.is_std_import(use_item) {
            return false;
        }
        
        // Check if it starts with a simple identifier (external crate) vs a complex path (local module)
        let first_segment = path_str.split("::").next().unwrap_or("");
        !first_segment.starts_with("crate") && 
        !first_segment.starts_with("super") && 
        !first_segment.starts_with("self") &&
        !path_str.starts_with("::")
    }
    
    fn use_tree_to_string(&self, tree: &UseTree) -> String {
        quote::quote!(#tree).to_string()
    }
    
    fn reconstruct_file_with_organized_imports(
        &self,
        original_content: &str,
        organized_imports: &[String],
        use_positions: &[usize],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = original_content.lines().collect();
        let mut result_lines = Vec::new();
        let mut import_region_inserted = false;
        let mut skip_import_lines: std::collections::HashSet<usize> = std::collections::HashSet::new();
        
        // Find lines that contain use statements to skip
        let syntax_tree: SynFile = syn::parse_str(original_content)?;
        
        // For simplicity, we'll replace the first use statement with all organized imports
        // and remove subsequent use statements
        let mut first_use_line: Option<usize> = None;
        
        for line in &syntax_tree.items {
            if let Item::Use(_) = line {
                // Find the line number of this use statement
                // This is simplified - in a real implementation, you'd need to track line numbers more carefully
                break;
            }
        }
        
        // Reconstruct file with organized imports
        let mut in_import_region = false;
        for (line_idx, line) in lines.iter().enumerate() {
            let line_trimmed = line.trim();
            
            // Detect import lines
            if line_trimmed.starts_with("use ") && line_trimmed.ends_with(";") {
                if !import_region_inserted {
                    // Insert all organized imports here
                    for import in organized_imports {
                        if !import.is_empty() {
                            result_lines.push(import.clone());
                        } else {
                            result_lines.push(String::new());
                        }
                    }
                    import_region_inserted = true;
                }
                // Skip the original import line
                continue;
            }
            
            result_lines.push(line.to_string());
        }
        
        Ok(result_lines.join("\n"))
    }
    
    pub fn create_import_fix(&self, content: &str) -> Result<Option<Fix>, Box<dyn std::error::Error>> {
        let organized = self.organize_imports(content)?;
        
        if organized == content {
            return Ok(None); // No changes needed
        }
        
        // Create a fix that replaces the entire content
        // In a more sophisticated implementation, you'd calculate specific ranges
        let fix = Fix {
            description: "Reorganize imports".to_string(),
            replacements: vec![Replacement {
                start: 0,
                end: content.len(),
                text: organized,
            }],
        };
        
        Ok(Some(fix))
    }
}

// Naming convention fixes
pub struct NamingConventionFixer;

impl NamingConventionFixer {
    pub fn new() -> Self {
        Self
    }
    
    pub fn create_snake_case_fix(&self, identifier: &str, location: &Location) -> Option<Fix> {
        let snake_case = self.to_snake_case(identifier);
        
        if snake_case == identifier {
            return None;
        }
        
        Some(Fix {
            description: format!("Convert '{}' to snake_case: '{}'", identifier, snake_case),
            replacements: vec![Replacement {
                start: location.column.saturating_sub(1),
                end: location.column + identifier.len() - 1,
                text: snake_case,
            }],
        })
    }
    
    pub fn create_pascal_case_fix(&self, identifier: &str, location: &Location) -> Option<Fix> {
        let pascal_case = self.to_pascal_case(identifier);
        
        if pascal_case == identifier {
            return None;
        }
        
        Some(Fix {
            description: format!("Convert '{}' to PascalCase: '{}'", identifier, pascal_case),
            replacements: vec![Replacement {
                start: location.column.saturating_sub(1),
                end: location.column + identifier.len() - 1,
                text: pascal_case,
            }],
        })
    }
    
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch.is_uppercase() {
                if !result.is_empty() && !result.ends_with('_') {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap_or(ch));
            } else {
                result.push(ch);
            }
        }
        
        result
    }
    
    fn to_pascal_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = true;
        
        for ch in s.chars() {
            if ch == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_uppercase().next().unwrap_or(ch));
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }
        
        result
    }
}

// Documentation template generator
pub struct DocTemplateGenerator;

impl DocTemplateGenerator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate_function_doc_fix(&self, fn_name: &str, location: &Location, has_params: bool, has_return: bool) -> Fix {
        let mut doc_lines = vec![
            "/// ".to_string(),
            format!("/// {}", self.generate_function_description(fn_name)),
        ];
        
        if has_params {
            doc_lines.push("///".to_string());
            doc_lines.push("/// # Arguments".to_string());
            doc_lines.push("///".to_string());
            doc_lines.push("/// * `param` - Description of the parameter".to_string());
        }
        
        if has_return {
            doc_lines.push("///".to_string());
            doc_lines.push("/// # Returns".to_string());
            doc_lines.push("///".to_string());
            doc_lines.push("/// Description of the return value".to_string());
        }
        
        doc_lines.push("///".to_string());
        doc_lines.push("/// # Examples".to_string());
        doc_lines.push("///".to_string());
        doc_lines.push("/// ```".to_string());
        doc_lines.push(format!("/// // Example usage of {}", fn_name));
        doc_lines.push("/// ```".to_string());
        
        let doc_text = doc_lines.join("\n") + "\n";
        
        Fix {
            description: format!("Add documentation template for function '{}'", fn_name),
            replacements: vec![Replacement {
                start: location.column.saturating_sub(1),
                end: location.column.saturating_sub(1),
                text: doc_text,
            }],
        }
    }
    
    pub fn generate_struct_doc_fix(&self, struct_name: &str, location: &Location) -> Fix {
        let doc_text = format!(
            "/// {}\n///\n/// # Examples\n///\n/// ```\n/// // Example usage of {}\n/// ```\n",
            self.generate_struct_description(struct_name),
            struct_name
        );
        
        Fix {
            description: format!("Add documentation template for struct '{}'", struct_name),
            replacements: vec![Replacement {
                start: location.column.saturating_sub(1),
                end: location.column.saturating_sub(1),
                text: doc_text,
            }],
        }
    }
    
    fn generate_function_description(&self, fn_name: &str) -> String {
        // Simple heuristic to generate meaningful descriptions
        if fn_name.starts_with("get_") {
            format!("Gets the {}.", &fn_name[4..].replace('_', " "))
        } else if fn_name.starts_with("set_") {
            format!("Sets the {}.", &fn_name[4..].replace('_', " "))
        } else if fn_name.starts_with("is_") {
            format!("Checks if {}.", &fn_name[3..].replace('_', " "))
        } else if fn_name.starts_with("has_") {
            format!("Checks if has {}.", &fn_name[4..].replace('_', " "))
        } else if fn_name.starts_with("create_") || fn_name.starts_with("new_") {
            format!("Creates a new {}.", fn_name.replace('_', " "))
        } else {
            format!("TODO: Add description for {}.", fn_name)
        }
    }
    
    fn generate_struct_description(&self, struct_name: &str) -> String {
        format!("Represents a {}.", struct_name.replace('_', " ").to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_naming_convention_fixer() {
        let fixer = NamingConventionFixer::new();
        
        assert_eq!(fixer.to_snake_case("CamelCase"), "camel_case");
        assert_eq!(fixer.to_snake_case("XMLHttpRequest"), "x_m_l_http_request");
        assert_eq!(fixer.to_pascal_case("snake_case"), "SnakeCase");
        assert_eq!(fixer.to_pascal_case("already_pascal"), "AlreadyPascal");
    }
    
    #[test]
    fn test_import_organizer() {
        let organizer = ImportOrganizer::new();
        let content = r#"
use std::collections::HashMap;
use serde::Serialize;
use crate::local_module;
use std::fs;

fn main() {}
"#;
        
        let organized = organizer.organize_imports(content).unwrap();
        assert!(organized.contains("use std::"));
        assert!(organized.contains("use serde::"));
        assert!(organized.contains("use crate::"));
    }
}