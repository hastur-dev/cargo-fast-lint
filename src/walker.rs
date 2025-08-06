use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub struct RustFileWalker {
    builder: WalkBuilder,
}

impl RustFileWalker {
    pub fn new() -> Self {
        let mut builder = WalkBuilder::new(".");
        builder
            .standard_filters(true)
            .add_custom_ignore_filename(".flignore");
            
        Self { builder }
    }
    
    pub fn walk(&self, path: &Path) -> impl Iterator<Item = PathBuf> {
        let mut builder = WalkBuilder::new(path);
        builder
            .standard_filters(true)
            .add_custom_ignore_filename(".flignore");
            
        builder.build()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .map_or(false, |ext| ext == "rs")
            })
            .map(|entry| entry.path().to_path_buf())
    }
}