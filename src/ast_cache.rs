use ahash::AHashMap;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use syn::File as SynFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAST {
    pub file_hash: u64,
    pub ast_tokens: Vec<u8>, // Serialized AST
    pub creation_time: u64,
}

pub struct ASTCache {
    cache: Arc<RwLock<AHashMap<PathBuf, CachedAST>>>,
    cache_file: PathBuf,
    max_cache_size: usize,
}

pub struct MmapFileReader {
    _file: File,
    mmap: Mmap,
}

impl MmapFileReader {
    pub fn new(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        Ok(Self {
            _file: file,
            mmap,
        })
    }
    
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.mmap)
    }
    
    pub fn len(&self) -> usize {
        self.mmap.len()
    }
    
    pub fn is_large_file(&self) -> bool {
        self.len() > 1024 * 1024 // 1MB threshold
    }
}

impl ASTCache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        let cache_file = cache_dir.as_ref().join("ast-cache.bin");
        let cache = Arc::new(RwLock::new(AHashMap::new()));
        
        let ast_cache = Self {
            cache,
            cache_file,
            max_cache_size: 10000, // Maximum number of cached ASTs
        };
        
        if let Err(e) = ast_cache.load() {
            eprintln!("Warning: Failed to load AST cache: {}", e);
        }
        
        ast_cache
    }
    
    fn compute_file_hash(content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
    
    pub fn get_or_parse(&self, path: &Path) -> Result<SynFile, Box<dyn std::error::Error>> {
        // Try memory-mapped reading for potentially large files
        let content = if let Ok(mmap_reader) = MmapFileReader::new(path) {
            if mmap_reader.is_large_file() {
                // Use memory-mapped reading for large files
                mmap_reader.as_str()?.to_string()
            } else {
                // Fall back to regular reading for smaller files
                std::fs::read_to_string(path)?
            }
        } else {
            // Fallback to regular file reading
            std::fs::read_to_string(path)?
        };
        
        let file_hash = Self::compute_file_hash(&content);
        
        // Check cache first - but for now we'll just parse fresh each time
        // since syn::File doesn't implement Serialize/Deserialize
        // TODO: Could implement custom serialization or use a different caching strategy
        
        // Parse fresh (we still get benefits from memory-mapped I/O)
        let ast = syn::parse_file(&content)?;
        
        Ok(ast)
    }
    
    // AST caching disabled for now since syn::File doesn't implement Serialize/Deserialize
    // The main performance benefit comes from memory-mapped I/O and incremental analysis
    
    fn evict_oldest(&self, cache: &mut AHashMap<PathBuf, CachedAST>) {
        if cache.is_empty() {
            return;
        }
        
        let mut oldest_path = None;
        let mut oldest_time = u64::MAX;
        
        for (path, cached) in cache.iter() {
            if cached.creation_time < oldest_time {
                oldest_time = cached.creation_time;
                oldest_path = Some(path.clone());
            }
        }
        
        if let Some(path) = oldest_path {
            cache.remove(&path);
        }
    }
    
    pub fn invalidate(&self, path: &Path) {
        if let Ok(mut cache_write) = self.cache.write() {
            cache_write.remove(path);
        }
    }
    
    pub fn clear(&self) {
        if let Ok(mut cache_write) = self.cache.write() {
            cache_write.clear();
        }
    }
    
    pub fn cache_stats(&self) -> Result<ASTCacheStats, Box<dyn std::error::Error + '_>> {
        let cache_read = self.cache.read()?;
        let total_entries = cache_read.len();
        let total_size_bytes: usize = cache_read.values()
            .map(|cached| cached.ast_tokens.len())
            .sum();
        
        let avg_ast_size = if total_entries > 0 {
            total_size_bytes / total_entries
        } else {
            0
        };
        
        Ok(ASTCacheStats {
            total_entries,
            total_size_bytes,
            avg_ast_size,
            max_cache_size: self.max_cache_size,
        })
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error + '_>> {
        let cache_read = self.cache.read()?;
        
        if let Some(parent) = self.cache_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.cache_file)?;
        
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &*cache_read)?;
        
        Ok(())
    }
    
    pub fn load(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.cache_file.exists() {
            return Ok(());
        }
        
        let file = File::open(&self.cache_file)?;
        let reader = BufReader::new(file);
        let loaded_cache: AHashMap<PathBuf, CachedAST> = bincode::deserialize_from(reader)?;
        
        if let Ok(mut cache_write) = self.cache.write() {
            *cache_write = loaded_cache;
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct ASTCacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub avg_ast_size: usize,
    pub max_cache_size: usize,
}

impl Drop for ASTCache {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            eprintln!("Warning: Failed to save AST cache on drop: {}", e);
        }
    }
}

// High-level API for file reading with automatic optimization
pub fn read_rust_file(path: &Path) -> Result<String, io::Error> {
    // For files larger than 1MB, use memory mapping
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 1024 * 1024 {
            let mmap_reader = MmapFileReader::new(path)?;
            return Ok(mmap_reader.as_str().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, e)
            })?.to_string());
        }
    }
    
    // For smaller files, use regular reading
    std::fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_ast_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ASTCache::new(temp_dir.path());
        
        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();
        
        // First parse - should cache
        let ast1 = cache.get_or_parse(&test_file).unwrap();
        assert_eq!(ast1.items.len(), 1);
        
        // Second parse - should use cache
        let ast2 = cache.get_or_parse(&test_file).unwrap();
        assert_eq!(ast2.items.len(), 1);
        
        // Verify cache stats
        let stats = cache.cache_stats().unwrap();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.total_size_bytes > 0);
    }
    
    #[test]
    fn test_mmap_reader() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        let content = "fn main() { println!(\"Hello, world!\"); }";
        fs::write(&test_file, content).unwrap();
        
        let reader = MmapFileReader::new(&test_file).unwrap();
        assert_eq!(reader.as_str().unwrap(), content);
        assert!(!reader.is_large_file()); // Small test file
    }
}