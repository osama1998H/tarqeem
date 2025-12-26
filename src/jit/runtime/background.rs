//! Background Compilation
//!
//! Provides non-blocking compilation by running JIT compilation in
//! a separate thread, allowing the main execution to continue.
//!
//! الترجمة في الخلفية
//! توفر ترجمة غير معرقلة عبر تشغيل الترجمة الفورية في خيط منفصل

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::ir::FuncId;
use crate::jit::profile::CompilationTier;

/// Request to compile a function in the background
#[derive(Debug, Clone)]
pub struct CompileRequest {
    /// Function to compile
    pub func_id: FuncId,

    /// Target tier for compilation
    pub tier: CompilationTier,

    /// Priority (higher = sooner)
    pub priority: u32,

    /// Timestamp when request was created
    pub created_at: std::time::Instant,
}

impl CompileRequest {
    /// Create a new compile request
    pub fn new(func_id: FuncId, tier: CompilationTier) -> Self {
        Self {
            func_id,
            tier,
            priority: 0,
            created_at: std::time::Instant::now(),
        }
    }

    /// Set priority (higher = compiled sooner)
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

impl PartialEq for CompileRequest {
    fn eq(&self, other: &Self) -> bool {
        self.func_id == other.func_id && self.tier == other.tier
    }
}

impl Eq for CompileRequest {}

/// Result of a background compilation
#[derive(Debug)]
pub struct CompileResult {
    /// Function that was compiled
    pub func_id: FuncId,

    /// Resulting tier (may differ from requested if fallback occurred)
    pub tier: CompilationTier,

    /// Whether compilation succeeded
    pub success: bool,

    /// Error message if compilation failed
    pub error: Option<String>,

    /// Time taken to compile in milliseconds
    pub compile_time_ms: u64,
}

impl CompileResult {
    /// Create a successful result
    pub fn success(func_id: FuncId, tier: CompilationTier, compile_time_ms: u64) -> Self {
        Self {
            func_id,
            tier,
            success: true,
            error: None,
            compile_time_ms,
        }
    }

    /// Create a failed result
    pub fn failure(func_id: FuncId, tier: CompilationTier, error: String) -> Self {
        Self {
            func_id,
            tier,
            success: false,
            error: Some(error),
            compile_time_ms: 0,
        }
    }
}

/// Shared state between main thread and background compiler
struct CompilerState {
    /// Queue of pending compilation requests
    queue: VecDeque<CompileRequest>,

    /// Completed compilation results waiting to be consumed
    results: VecDeque<CompileResult>,

    /// Maximum queue size
    max_queue_size: usize,
}

impl CompilerState {
    fn new(max_queue_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            results: VecDeque::new(),
            max_queue_size,
        }
    }
}

/// Background compiler that runs in a separate thread
pub struct BackgroundCompiler {
    /// Shared state
    state: Arc<Mutex<CompilerState>>,

    /// Flag to signal shutdown
    shutdown: Arc<AtomicBool>,

    /// Background thread handle
    thread_handle: Option<JoinHandle<()>>,

    /// Statistics
    requests_submitted: AtomicU64,
    requests_completed: AtomicU64,
    requests_dropped: AtomicU64,

    /// Whether background compilation is enabled
    enabled: bool,
}

impl BackgroundCompiler {
    /// Create a new background compiler
    pub fn new(max_queue_size: usize) -> Self {
        let state = Arc::new(Mutex::new(CompilerState::new(max_queue_size)));
        let shutdown = Arc::new(AtomicBool::new(false));

        Self {
            state,
            shutdown,
            thread_handle: None,
            requests_submitted: AtomicU64::new(0),
            requests_completed: AtomicU64::new(0),
            requests_dropped: AtomicU64::new(0),
            enabled: true,
        }
    }

    /// Create a disabled background compiler (synchronous mode)
    pub fn disabled() -> Self {
        let mut compiler = Self::new(0);
        compiler.enabled = false;
        compiler
    }

    /// Start the background compilation thread
    ///
    /// The compile_fn is called for each compilation request
    pub fn start<F>(&mut self, compile_fn: F)
    where
        F: Fn(CompileRequest) -> CompileResult + Send + 'static,
    {
        if !self.enabled {
            return;
        }

        let state = Arc::clone(&self.state);
        let shutdown = Arc::clone(&self.shutdown);

        self.thread_handle = Some(thread::spawn(move || {
            Self::compiler_thread(state, shutdown, compile_fn);
        }));
    }

    /// The main compilation thread loop
    fn compiler_thread<F>(state: Arc<Mutex<CompilerState>>, shutdown: Arc<AtomicBool>, compile_fn: F)
    where
        F: Fn(CompileRequest) -> CompileResult,
    {
        loop {
            // Check for shutdown
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Get next request from queue
            let request = {
                let mut state = state.lock().unwrap();
                state.queue.pop_front()
            };

            if let Some(request) = request {
                // Compile the function
                let result = compile_fn(request);

                // Store the result
                let mut state = state.lock().unwrap();
                state.results.push_back(result);
            } else {
                // No work to do, sleep briefly
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    /// Submit a compilation request
    pub fn submit(&self, request: CompileRequest) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.lock().unwrap();

        // Check if already in queue
        if state.queue.iter().any(|r| r.func_id == request.func_id) {
            return false;
        }

        // Check queue capacity
        if state.queue.len() >= state.max_queue_size {
            self.requests_dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Insert based on priority (higher priority first)
        let pos = state
            .queue
            .iter()
            .position(|r| r.priority < request.priority)
            .unwrap_or(state.queue.len());

        state.queue.insert(pos, request);
        self.requests_submitted.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Take all completed results
    pub fn take_results(&self) -> Vec<CompileResult> {
        let mut state = self.state.lock().unwrap();
        let results: Vec<_> = state.results.drain(..).collect();
        self.requests_completed
            .fetch_add(results.len() as u64, Ordering::Relaxed);
        results
    }

    /// Check if there are pending requests
    pub fn has_pending(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.queue.is_empty()
    }

    /// Check if there are completed results
    pub fn has_results(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.results.is_empty()
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.queue.len()
    }

    /// Clear all pending requests
    pub fn clear_pending(&self) {
        let mut state = self.state.lock().unwrap();
        let dropped = state.queue.len();
        state.queue.clear();
        self.requests_dropped
            .fetch_add(dropped as u64, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> BackgroundCompilerStats {
        let state = self.state.lock().unwrap();
        BackgroundCompilerStats {
            enabled: self.enabled,
            queue_size: state.queue.len(),
            pending_results: state.results.len(),
            requests_submitted: self.requests_submitted.load(Ordering::Relaxed),
            requests_completed: self.requests_completed.load(Ordering::Relaxed),
            requests_dropped: self.requests_dropped.load(Ordering::Relaxed),
        }
    }

    /// Shutdown the background compiler
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        if let Some(handle) = self.thread_handle.take() {
            // Wait for thread to finish (with timeout)
            let _ = handle.join();
        }
    }

    /// Check if background compilation is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Drop for BackgroundCompiler {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Statistics for background compilation
#[derive(Debug, Clone)]
pub struct BackgroundCompilerStats {
    /// Whether background compilation is enabled
    pub enabled: bool,

    /// Current queue size
    pub queue_size: usize,

    /// Number of results waiting to be consumed
    pub pending_results: usize,

    /// Total requests submitted
    pub requests_submitted: u64,

    /// Total requests completed
    pub requests_completed: u64,

    /// Total requests dropped (due to queue full)
    pub requests_dropped: u64,
}

impl std::fmt::Display for BackgroundCompilerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Background Compiler Statistics / إحصائيات الترجمة في الخلفية")?;
        writeln!(f, "==============================================================")?;
        writeln!(
            f,
            "Enabled / مُفعَّل: {}",
            if self.enabled { "yes / نعم" } else { "no / لا" }
        )?;
        writeln!(f, "Queue size / حجم الطابور: {}", self.queue_size)?;
        writeln!(
            f,
            "Pending results / نتائج معلقة: {}",
            self.pending_results
        )?;
        writeln!(
            f,
            "Requests submitted / الطلبات المُقدمة: {}",
            self.requests_submitted
        )?;
        writeln!(
            f,
            "Requests completed / الطلبات المُكتملة: {}",
            self.requests_completed
        )?;
        writeln!(
            f,
            "Requests dropped / الطلبات المُهملة: {}",
            self.requests_dropped
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_request_creation() {
        let request = CompileRequest::new(
            FuncId("test".to_string()),
            CompilationTier::BaselineCompiled,
        )
        .with_priority(5);

        assert_eq!(request.func_id.0, "test");
        assert_eq!(request.tier, CompilationTier::BaselineCompiled);
        assert_eq!(request.priority, 5);
    }

    #[test]
    fn test_compile_result() {
        let success = CompileResult::success(
            FuncId("test".to_string()),
            CompilationTier::Optimized,
            100,
        );
        assert!(success.success);
        assert!(success.error.is_none());
        assert_eq!(success.compile_time_ms, 100);

        let failure = CompileResult::failure(
            FuncId("test".to_string()),
            CompilationTier::Optimized,
            "Test error".to_string(),
        );
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }

    #[test]
    fn test_background_compiler_submit() {
        let compiler = BackgroundCompiler::new(10);

        let request = CompileRequest::new(
            FuncId("func1".to_string()),
            CompilationTier::BaselineCompiled,
        );
        assert!(compiler.submit(request));

        // Duplicate should be rejected
        let duplicate = CompileRequest::new(
            FuncId("func1".to_string()),
            CompilationTier::Optimized,
        );
        assert!(!compiler.submit(duplicate));

        // Different function should be accepted
        let request2 = CompileRequest::new(
            FuncId("func2".to_string()),
            CompilationTier::BaselineCompiled,
        );
        assert!(compiler.submit(request2));

        assert_eq!(compiler.pending_count(), 2);
    }

    #[test]
    fn test_background_compiler_priority() {
        let compiler = BackgroundCompiler::new(10);

        // Submit low priority first
        let low = CompileRequest::new(FuncId("low".to_string()), CompilationTier::BaselineCompiled)
            .with_priority(1);
        compiler.submit(low);

        // Submit high priority second
        let high =
            CompileRequest::new(FuncId("high".to_string()), CompilationTier::BaselineCompiled)
                .with_priority(10);
        compiler.submit(high);

        // High priority should be first in queue
        let state = compiler.state.lock().unwrap();
        assert_eq!(state.queue[0].func_id.0, "high");
        assert_eq!(state.queue[1].func_id.0, "low");
    }

    #[test]
    fn test_background_compiler_queue_limit() {
        let compiler = BackgroundCompiler::new(2);

        let r1 = CompileRequest::new(FuncId("f1".to_string()), CompilationTier::BaselineCompiled);
        let r2 = CompileRequest::new(FuncId("f2".to_string()), CompilationTier::BaselineCompiled);
        let r3 = CompileRequest::new(FuncId("f3".to_string()), CompilationTier::BaselineCompiled);

        assert!(compiler.submit(r1));
        assert!(compiler.submit(r2));
        assert!(!compiler.submit(r3)); // Should be rejected

        let stats = compiler.stats();
        assert_eq!(stats.requests_dropped, 1);
    }

    #[test]
    fn test_background_compiler_disabled() {
        let compiler = BackgroundCompiler::disabled();

        let request = CompileRequest::new(
            FuncId("test".to_string()),
            CompilationTier::BaselineCompiled,
        );
        assert!(!compiler.submit(request));
        assert!(!compiler.is_enabled());
    }

    #[test]
    fn test_background_compiler_clear_pending() {
        let compiler = BackgroundCompiler::new(10);

        for i in 0..5 {
            let request = CompileRequest::new(
                FuncId(format!("func{}", i)),
                CompilationTier::BaselineCompiled,
            );
            compiler.submit(request);
        }

        assert_eq!(compiler.pending_count(), 5);

        compiler.clear_pending();

        assert_eq!(compiler.pending_count(), 0);
        assert_eq!(compiler.stats().requests_dropped, 5);
    }

    #[test]
    fn test_background_compiler_stats() {
        let compiler = BackgroundCompiler::new(10);

        let stats = compiler.stats();
        assert!(stats.enabled);
        assert_eq!(stats.queue_size, 0);
        assert_eq!(stats.requests_submitted, 0);
    }
}
