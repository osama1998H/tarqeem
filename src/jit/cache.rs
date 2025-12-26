//! JIT Code Cache
//!
//! This module provides a cache for compiled native functions.
//! It manages the lifecycle of JIT-compiled code and provides
//! fast lookup for function execution.

use std::collections::HashMap;
use std::sync::Arc;

use super::profile::CompilationTier;

/// Metadata for a compiled function
#[derive(Debug, Clone)]
pub struct CompiledFunctionInfo {
    /// Function name
    pub name: String,

    /// Compilation tier
    pub tier: CompilationTier,

    /// Size of generated code in bytes
    pub code_size: usize,

    /// Time taken to compile in milliseconds
    pub compile_time_ms: u64,
}

/// A compiled function stored in the cache
pub struct CompiledFunction {
    /// Function metadata
    pub info: CompiledFunctionInfo,

    /// The compiled code (opaque to this module)
    /// This is managed by Cranelift's JITModule
    #[cfg(feature = "jit")]
    pub func_id: Option<cranelift_module::FuncId>,

    /// Placeholder when JIT feature is disabled
    #[cfg(not(feature = "jit"))]
    pub func_id: Option<()>,
}

impl CompiledFunction {
    /// Create a new compiled function entry
    pub fn new(
        name: String,
        tier: CompilationTier,
        code_size: usize,
        compile_time_ms: u64,
    ) -> Self {
        Self {
            info: CompiledFunctionInfo {
                name,
                tier,
                code_size,
                compile_time_ms,
            },
            func_id: None,
        }
    }

    /// Create with Cranelift function ID
    #[cfg(feature = "jit")]
    pub fn with_func_id(
        name: String,
        tier: CompilationTier,
        code_size: usize,
        compile_time_ms: u64,
        func_id: cranelift_module::FuncId,
    ) -> Self {
        Self {
            info: CompiledFunctionInfo {
                name,
                tier,
                code_size,
                compile_time_ms,
            },
            func_id: Some(func_id),
        }
    }
}

impl std::fmt::Debug for CompiledFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledFunction")
            .field("info", &self.info)
            .field("has_func_id", &self.func_id.is_some())
            .finish()
    }
}

/// Cache for compiled functions with LRU eviction
#[derive(Debug, Default)]
pub struct CodeCache {
    /// Map from function name to compiled function
    functions: HashMap<String, Arc<CompiledFunction>>,

    /// Access order for LRU eviction (most recent last)
    access_order: Vec<String>,

    /// Total size of cached code in bytes
    total_code_size: usize,

    /// Maximum cache size in bytes
    max_size: usize,

    /// Number of cache hits
    hits: u64,

    /// Number of cache misses
    misses: u64,

    /// Number of evictions
    evictions: u64,
}

impl CodeCache {
    /// Create a new code cache with default size (64 MB)
    pub fn new() -> Self {
        Self::with_max_size(64 * 1024 * 1024)
    }

    /// Create a new code cache with specified maximum size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            functions: HashMap::new(),
            access_order: Vec::new(),
            total_code_size: 0,
            max_size,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Insert a compiled function into the cache
    ///
    /// If the cache is full, LRU entries are evicted to make room.
    pub fn insert(&mut self, func: CompiledFunction) -> bool {
        let name = func.info.name.clone();
        let code_size = func.info.code_size;

        // If the function is too large for the cache, reject it
        if code_size > self.max_size {
            return false;
        }

        // Remove old version if exists
        if let Some(old) = self.functions.remove(&name) {
            self.total_code_size -= old.info.code_size;
            self.access_order.retain(|n| n != &name);
        }

        // Evict LRU entries until we have enough space
        while self.total_code_size + code_size > self.max_size && !self.access_order.is_empty() {
            self.evict_lru();
        }

        // Insert the new function
        self.total_code_size += code_size;
        self.functions.insert(name.clone(), Arc::new(func));
        self.access_order.push(name);
        true
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if let Some(lru_name) = self.access_order.first().cloned() {
            if let Some(func) = self.functions.remove(&lru_name) {
                self.total_code_size -= func.info.code_size;
                self.evictions += 1;
            }
            self.access_order.remove(0);
        }
    }

    /// Get a compiled function from the cache
    ///
    /// Updates the access order for LRU tracking.
    pub fn get(&mut self, name: &str) -> Option<Arc<CompiledFunction>> {
        if let Some(func) = self.functions.get(name).cloned() {
            self.hits += 1;
            // Move to end (most recently used)
            self.touch(name);
            Some(func)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Update access order for a function (mark as recently used)
    fn touch(&mut self, name: &str) {
        if let Some(pos) = self.access_order.iter().position(|n| n == name) {
            self.access_order.remove(pos);
            self.access_order.push(name.to_string());
        }
    }

    /// Check if a function is in the cache (without affecting hit/miss stats)
    pub fn contains(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get the tier of a cached function
    pub fn get_tier(&self, name: &str) -> Option<CompilationTier> {
        self.functions.get(name).map(|f| f.info.tier)
    }

    /// Remove a function from the cache
    pub fn remove(&mut self, name: &str) -> Option<Arc<CompiledFunction>> {
        if let Some(func) = self.functions.remove(name) {
            self.total_code_size -= func.info.code_size;
            self.access_order.retain(|n| n != name);
            Some(func)
        } else {
            None
        }
    }

    /// Clear all cached functions
    pub fn clear(&mut self) {
        self.functions.clear();
        self.access_order.clear();
        self.total_code_size = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            function_count: self.functions.len(),
            total_code_size: self.total_code_size,
            max_size: self.max_size,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Get number of cached functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Iterate over all cached functions
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<CompiledFunction>)> {
        self.functions.iter()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of functions in cache
    pub function_count: usize,

    /// Total size of cached code
    pub total_code_size: usize,

    /// Maximum cache size
    pub max_size: usize,

    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Number of LRU evictions
    pub evictions: u64,

    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Code Cache Statistics / إحصائيات ذاكرة التخزين")?;
        writeln!(f, "================================================")?;
        writeln!(f, "Functions / الدوال: {}", self.function_count)?;
        writeln!(
            f,
            "Code size / حجم الكود: {} bytes ({:.2} KB)",
            self.total_code_size,
            self.total_code_size as f64 / 1024.0
        )?;
        writeln!(
            f,
            "Max size / الحد الأقصى: {} bytes ({:.2} MB)",
            self.max_size,
            self.max_size as f64 / (1024.0 * 1024.0)
        )?;
        writeln!(f, "Hits / الإصابات: {}", self.hits)?;
        writeln!(f, "Misses / الإخفاقات: {}", self.misses)?;
        writeln!(f, "Evictions / الإخلاءات: {}", self.evictions)?;
        writeln!(f, "Hit rate / نسبة الإصابة: {:.2}%", self.hit_rate * 100.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = CodeCache::new();

        let func = CompiledFunction::new(
            "test_func".to_string(),
            CompilationTier::BaselineCompiled,
            1024,
            5,
        );

        assert!(cache.insert(func));
        assert!(cache.contains("test_func"));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get("test_func");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().info.name, "test_func");
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = CodeCache::new();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = CodeCache::new();

        let func = CompiledFunction::new(
            "test_func".to_string(),
            CompilationTier::BaselineCompiled,
            1024,
            5,
        );

        cache.insert(func);
        assert_eq!(cache.len(), 1);

        cache.remove("test_func");
        assert_eq!(cache.len(), 0);
        assert!(!cache.contains("test_func"));
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = CodeCache::new();

        let func = CompiledFunction::new(
            "func1".to_string(),
            CompilationTier::BaselineCompiled,
            1024,
            5,
        );
        cache.insert(func);

        // Generate some hits and misses
        cache.get("func1"); // hit
        cache.get("func1"); // hit
        cache.get("nonexistent"); // miss

        let stats = cache.stats();
        assert_eq!(stats.function_count, 1);
        assert_eq!(stats.total_code_size, 1024);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = CodeCache::with_max_size(2048);

        let func1 = CompiledFunction::new(
            "func1".to_string(),
            CompilationTier::BaselineCompiled,
            1024,
            5,
        );
        assert!(cache.insert(func1));

        let func2 = CompiledFunction::new(
            "func2".to_string(),
            CompilationTier::BaselineCompiled,
            1024,
            5,
        );
        assert!(cache.insert(func2));

        // Cache is full, but LRU eviction should allow insert
        let func3 = CompiledFunction::new(
            "func3".to_string(),
            CompilationTier::BaselineCompiled,
            1024,
            5,
        );
        assert!(cache.insert(func3));

        // func1 (LRU) should have been evicted
        assert!(!cache.contains("func1"));
        assert!(cache.contains("func2"));
        assert!(cache.contains("func3"));

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cache_too_large_function() {
        let mut cache = CodeCache::with_max_size(1024);

        // Function larger than cache should be rejected
        let func = CompiledFunction::new(
            "large_func".to_string(),
            CompilationTier::BaselineCompiled,
            2048,
            5,
        );
        assert!(!cache.insert(func));
    }

    #[test]
    fn test_cache_tier_lookup() {
        let mut cache = CodeCache::new();

        let func = CompiledFunction::new(
            "test_func".to_string(),
            CompilationTier::Optimized,
            2048,
            50,
        );
        cache.insert(func);

        assert_eq!(
            cache.get_tier("test_func"),
            Some(CompilationTier::Optimized)
        );
        assert_eq!(cache.get_tier("unknown"), None);
    }
}
