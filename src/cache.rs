use crate::rules::Issue;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: u64,
    pub hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysis {
    pub metadata: FileMetadata,
    pub issues: Vec<Issue>,
    pub ast_hash: Option<u64>,
}

#[derive(Debug, Default)]
pub struct AnalysisCache {
    cache: AHashMap<PathBuf, CachedAnalysis>,
    cache_file: PathBuf,
    dirty: bool,
}

impl AnalysisCache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        let cache_file = cache_dir.as_ref().join("cargo-fl-cache.bin");
        let mut cache = Self {
            cache: AHashMap::new(),
            cache_file,
            dirty: false,
        };
        
        if let Err(e) = cache.load() {
            eprintln!("Warning: Failed to load cache: {}", e);
        }
        
        cache
    }
    
    pub fn get_metadata(path: &Path) -> Result<FileMetadata, std::io::Error> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Simple hash based on path, size, and modification time
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        path.hash(&mut hasher);
        size.hash(&mut hasher);
        modified.hash(&mut hasher);
        let hash = hasher.finish();
        
        Ok(FileMetadata {
            path: path.to_path_buf(),
            size,
            modified,
            hash,
        })
    }
    
    pub fn is_file_changed(&self, path: &Path) -> Result<bool, std::io::Error> {
        let current_metadata = Self::get_metadata(path)?;
        
        if let Some(cached) = self.cache.get(path) {
            Ok(cached.metadata.hash != current_metadata.hash)
        } else {
            Ok(true) // File not in cache, consider it changed
        }
    }
    
    pub fn get_cached_analysis(&self, path: &Path) -> Option<&CachedAnalysis> {
        self.cache.get(path)
    }
    
    pub fn store_analysis(&mut self, path: PathBuf, issues: Vec<Issue>, ast_hash: Option<u64>) -> Result<(), std::io::Error> {
        let metadata = Self::get_metadata(&path)?;
        
        let cached = CachedAnalysis {
            metadata,
            issues,
            ast_hash,
        };
        
        self.cache.insert(path, cached);
        self.dirty = true;
        
        Ok(())
    }
    
    pub fn remove_file(&mut self, path: &Path) {
        if self.cache.remove(path).is_some() {
            self.dirty = true;
        }
    }
    
    pub fn cleanup_stale_entries(&mut self) {
        let mut stale_paths = Vec::new();
        
        for (path, cached) in &self.cache {
            if !path.exists() {
                stale_paths.push(path.clone());
            } else if let Ok(current_meta) = Self::get_metadata(path) {
                if current_meta.hash != cached.metadata.hash {
                    stale_paths.push(path.clone());
                }
            }
        }
        
        for path in stale_paths {
            self.cache.remove(&path);
            self.dirty = true;
        }
    }
    
    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.dirty {
            return Ok(());
        }
        
        if let Some(parent) = self.cache_file.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let serialized = bincode::serialize(&self.cache)?;
        fs::write(&self.cache_file, serialized)?;
        self.dirty = false;
        
        Ok(())
    }
    
    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.cache_file.exists() {
            return Ok(());
        }
        
        let data = fs::read(&self.cache_file)?;
        self.cache = bincode::deserialize(&data)?;
        self.dirty = false;
        
        Ok(())
    }
    
    pub fn cache_stats(&self) -> CacheStats {
        let total_files = self.cache.len();
        let total_issues = self.cache.values().map(|c| c.issues.len()).sum();
        let cache_size_bytes = bincode::serialized_size(&self.cache).unwrap_or(0);
        
        CacheStats {
            total_files,
            total_issues,
            cache_size_bytes,
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_issues: usize,
    pub cache_size_bytes: u64,
}

impl Drop for AnalysisCache {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            eprintln!("Warning: Failed to save cache on drop: {}", e);
        }
    }
}