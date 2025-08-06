use std::collections::HashMap;

fn demonstrate_rules() -> Result<String, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    map.insert("key", "value");
    
    // This will trigger unwrap_usage rule
    let value = map.get("key").unwrap();
    
    // This will trigger todo_macros rule
    todo!("Implement this function");
    
    // This will trigger must_use violation (unused result)
    vec![1, 2, 3].iter().map(|x| x * 2);
    
    // This will trigger anti_patterns rule
    let s = "hello".to_string();
    let v = vec![1, 2, 3];
    let cloned = v.clone(); // Unnecessary clone
    
    // This will trigger must_use (Result not used)
    std::fs::write("/tmp/test", "content");
    
    Ok(value.to_string())
}