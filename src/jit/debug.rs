//! JIT Debugging Support
//!
//! Provides utilities for debugging JIT-compiled code including
//! disassembly, code dumps, and execution tracing.
//!
//! دعم التصحيح للترجمة الفورية
//! يوفر أدوات لتصحيح الكود المترجم فورياً بما في ذلك
//! التفكيك، تفريغ الكود، وتتبع التنفيذ

use std::collections::HashMap;
use std::fmt;

use super::profile::CompilationTier;

/// Debug information for a compiled function
#[derive(Debug, Clone)]
pub struct FunctionDebugInfo {
    /// Function name
    pub name: String,

    /// Compilation tier
    pub tier: CompilationTier,

    /// Source file path (if available)
    pub source_file: Option<String>,

    /// Source line mappings (native offset -> source line)
    pub line_mappings: Vec<LineMapping>,

    /// Local variable information
    pub locals: Vec<LocalVarInfo>,

    /// Generated code size in bytes
    pub code_size: usize,
}

impl FunctionDebugInfo {
    /// Create new debug info for a function
    pub fn new(name: String, tier: CompilationTier) -> Self {
        Self {
            name,
            tier,
            source_file: None,
            line_mappings: Vec::new(),
            locals: Vec::new(),
            code_size: 0,
        }
    }

    /// Set source file
    pub fn with_source_file(mut self, path: String) -> Self {
        self.source_file = Some(path);
        self
    }

    /// Add a line mapping
    pub fn add_line_mapping(&mut self, native_offset: usize, source_line: u32) {
        self.line_mappings.push(LineMapping {
            native_offset,
            source_line,
        });
    }

    /// Add local variable info
    pub fn add_local(&mut self, local: LocalVarInfo) {
        self.locals.push(local);
    }

    /// Get source line for a native offset
    pub fn get_source_line(&self, native_offset: usize) -> Option<u32> {
        // Find the closest mapping at or before the offset
        let mut best_match = None;
        for mapping in &self.line_mappings {
            if mapping.native_offset <= native_offset {
                best_match = Some(mapping.source_line);
            }
        }
        best_match
    }
}

/// Mapping from native code offset to source line
#[derive(Debug, Clone)]
pub struct LineMapping {
    /// Offset into native code
    pub native_offset: usize,

    /// Source line number
    pub source_line: u32,
}

/// Information about a local variable in compiled code
#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    /// Variable name
    pub name: String,

    /// Type name
    pub type_name: String,

    /// Stack slot offset (negative for locals, positive for args)
    pub stack_offset: i32,

    /// Scope start (native offset)
    pub scope_start: usize,

    /// Scope end (native offset)
    pub scope_end: usize,
}

impl LocalVarInfo {
    /// Create new local variable info
    pub fn new(name: String, type_name: String, stack_offset: i32) -> Self {
        Self {
            name,
            type_name,
            stack_offset,
            scope_start: 0,
            scope_end: usize::MAX,
        }
    }

    /// Set scope range
    pub fn with_scope(mut self, start: usize, end: usize) -> Self {
        self.scope_start = start;
        self.scope_end = end;
        self
    }

    /// Check if variable is in scope at offset
    pub fn is_in_scope(&self, offset: usize) -> bool {
        offset >= self.scope_start && offset < self.scope_end
    }
}

/// JIT debug information manager
#[derive(Debug, Default)]
pub struct JitDebugInfo {
    /// Debug info for each compiled function
    functions: HashMap<String, FunctionDebugInfo>,

    /// Whether debug info collection is enabled
    enabled: bool,

    /// Verbosity level
    verbosity: DebugVerbosity,
}

impl JitDebugInfo {
    /// Create new debug info manager
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            enabled: false,
            verbosity: DebugVerbosity::Normal,
        }
    }

    /// Enable debug info collection
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable debug info collection
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set verbosity level
    pub fn set_verbosity(&mut self, level: DebugVerbosity) {
        self.verbosity = level;
    }

    /// Register debug info for a function
    pub fn register(&mut self, info: FunctionDebugInfo) {
        if self.enabled {
            self.functions.insert(info.name.clone(), info);
        }
    }

    /// Get debug info for a function
    pub fn get(&self, func_name: &str) -> Option<&FunctionDebugInfo> {
        self.functions.get(func_name)
    }

    /// Remove debug info for a function
    pub fn remove(&mut self, func_name: &str) {
        self.functions.remove(func_name);
    }

    /// Clear all debug info
    pub fn clear(&mut self) {
        self.functions.clear();
    }

    /// Get all registered functions
    pub fn function_names(&self) -> impl Iterator<Item = &String> {
        self.functions.keys()
    }

    /// Generate a debug dump for a function
    pub fn dump_function(&self, func_name: &str) -> Option<String> {
        let info = self.functions.get(func_name)?;

        let mut output = String::new();
        output.push_str(&format!(
            "=== Debug Info for {} / معلومات التصحيح لـ {} ===\n",
            func_name, func_name
        ));
        output.push_str(&format!("Tier / المستوى: {:?}\n", info.tier));
        output.push_str(&format!(
            "Code size / حجم الكود: {} bytes\n",
            info.code_size
        ));

        if let Some(ref source) = info.source_file {
            output.push_str(&format!("Source file / ملف المصدر: {}\n", source));
        }

        if !info.locals.is_empty() {
            output.push_str("\nLocal Variables / المتغيرات المحلية:\n");
            for local in &info.locals {
                output.push_str(&format!(
                    "  {}: {} (offset: {})\n",
                    local.name, local.type_name, local.stack_offset
                ));
            }
        }

        if !info.line_mappings.is_empty() {
            output.push_str("\nLine Mappings / تعيينات الأسطر:\n");
            for mapping in &info.line_mappings {
                output.push_str(&format!(
                    "  0x{:04x} -> line {}\n",
                    mapping.native_offset, mapping.source_line
                ));
            }
        }

        Some(output)
    }
}

/// Debug verbosity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugVerbosity {
    /// Minimal output
    Quiet,

    /// Normal output
    #[default]
    Normal,

    /// Verbose output
    Verbose,

    /// Maximum verbosity for debugging the JIT itself
    Trace,
}

/// Execution trace entry
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Function name
    pub function: String,

    /// Native offset within function
    pub offset: usize,

    /// Timestamp (nanoseconds since trace start)
    pub timestamp_ns: u64,

    /// Additional data
    pub data: TraceData,
}

/// Data associated with a trace entry
#[derive(Debug, Clone)]
pub enum TraceData {
    /// Function entry
    FunctionEntry,

    /// Function exit with optional return value description
    FunctionExit(Option<String>),

    /// Block entry
    BlockEntry(u32),

    /// Branch taken
    BranchTaken { target: u32, condition: bool },

    /// Loop iteration
    LoopIteration { loop_id: u32, iteration: u64 },

    /// Value computed
    ValueComputed { var: String, value: String },

    /// Memory access
    MemoryAccess { address: usize, is_write: bool },

    /// Deoptimization
    Deoptimization { reason: String },

    /// Custom event
    Custom(String),
}

impl fmt::Display for TraceData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceData::FunctionEntry => write!(f, "ENTER"),
            TraceData::FunctionExit(ret) => match ret {
                Some(v) => write!(f, "EXIT -> {}", v),
                None => write!(f, "EXIT"),
            },
            TraceData::BlockEntry(id) => write!(f, "BLOCK {}", id),
            TraceData::BranchTaken { target, condition } => {
                write!(f, "BRANCH {} ({})", target, condition)
            }
            TraceData::LoopIteration { loop_id, iteration } => {
                write!(f, "LOOP {} iter {}", loop_id, iteration)
            }
            TraceData::ValueComputed { var, value } => {
                write!(f, "{} = {}", var, value)
            }
            TraceData::MemoryAccess { address, is_write } => {
                let op = if *is_write { "WRITE" } else { "READ" };
                write!(f, "{} @{:#x}", op, address)
            }
            TraceData::Deoptimization { reason } => {
                write!(f, "DEOPT: {}", reason)
            }
            TraceData::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

/// Execution tracer for debugging
#[derive(Debug)]
pub struct ExecutionTracer {
    /// Trace entries
    entries: Vec<TraceEntry>,

    /// Whether tracing is active
    active: bool,

    /// Maximum entries to keep
    max_entries: usize,

    /// Start timestamp
    start_time: std::time::Instant,
}

impl Default for ExecutionTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionTracer {
    /// Create a new tracer
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            active: false,
            max_entries: 10_000,
            start_time: std::time::Instant::now(),
        }
    }

    /// Set maximum entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Start tracing
    pub fn start(&mut self) {
        self.active = true;
        self.start_time = std::time::Instant::now();
        self.entries.clear();
    }

    /// Stop tracing
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Check if tracing is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Record a trace entry
    pub fn record(&mut self, function: String, offset: usize, data: TraceData) {
        if !self.active {
            return;
        }

        let timestamp_ns = self.start_time.elapsed().as_nanos() as u64;

        // Evict oldest entries if at capacity
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }

        self.entries.push(TraceEntry {
            function,
            offset,
            timestamp_ns,
            data,
        });
    }

    /// Get all trace entries
    pub fn entries(&self) -> &[TraceEntry] {
        &self.entries
    }

    /// Clear trace entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Format trace as text
    pub fn format_trace(&self) -> String {
        let mut output = String::new();
        output.push_str("=== Execution Trace / تتبع التنفيذ ===\n");
        output.push_str(&format!("Entries / المدخلات: {}\n\n", self.entries.len()));

        for entry in &self.entries {
            output.push_str(&format!(
                "[{:>10}ns] {}+0x{:04x}: {}\n",
                entry.timestamp_ns, entry.function, entry.offset, entry.data
            ));
        }

        output
    }
}

/// Code dumper for debugging compiled code
#[derive(Debug, Default)]
pub struct CodeDumper {
    /// Format for output
    format: DumpFormat,
}

impl CodeDumper {
    /// Create new code dumper
    pub fn new() -> Self {
        Self {
            format: DumpFormat::Hex,
        }
    }

    /// Set output format
    pub fn with_format(mut self, format: DumpFormat) -> Self {
        self.format = format;
        self
    }

    /// Dump code bytes
    pub fn dump(&self, code: &[u8], base_address: usize) -> String {
        match self.format {
            DumpFormat::Hex => self.dump_hex(code, base_address),
            DumpFormat::Binary => self.dump_binary(code, base_address),
        }
    }

    fn dump_hex(&self, code: &[u8], base_address: usize) -> String {
        let mut output = String::new();
        output.push_str("=== Code Dump (Hex) / تفريغ الكود (سداسي عشري) ===\n");

        for (i, chunk) in code.chunks(16).enumerate() {
            let addr = base_address + i * 16;
            output.push_str(&format!("{:08x}:  ", addr));

            // Hex bytes
            for (j, byte) in chunk.iter().enumerate() {
                if j == 8 {
                    output.push(' ');
                }
                output.push_str(&format!("{:02x} ", byte));
            }

            // Padding if less than 16 bytes
            for j in chunk.len()..16 {
                if j == 8 {
                    output.push(' ');
                }
                output.push_str("   ");
            }

            // ASCII representation
            output.push_str(" |");
            for byte in chunk {
                if *byte >= 0x20 && *byte < 0x7f {
                    output.push(*byte as char);
                } else {
                    output.push('.');
                }
            }
            output.push_str("|\n");
        }

        output
    }

    fn dump_binary(&self, code: &[u8], base_address: usize) -> String {
        let mut output = String::new();
        output.push_str("=== Code Dump (Binary) / تفريغ الكود (ثنائي) ===\n");

        for (i, byte) in code.iter().enumerate() {
            let addr = base_address + i;
            output.push_str(&format!("{:08x}:  {:08b}\n", addr, byte));
        }

        output
    }
}

/// Output format for code dumps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DumpFormat {
    /// Hexadecimal dump
    #[default]
    Hex,

    /// Binary dump
    Binary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_debug_info() {
        let mut info =
            FunctionDebugInfo::new("test".to_string(), CompilationTier::BaselineCompiled);

        info.add_line_mapping(0, 1);
        info.add_line_mapping(10, 5);
        info.add_line_mapping(20, 10);

        assert_eq!(info.get_source_line(0), Some(1));
        assert_eq!(info.get_source_line(5), Some(1));
        assert_eq!(info.get_source_line(10), Some(5));
        assert_eq!(info.get_source_line(15), Some(5));
        assert_eq!(info.get_source_line(25), Some(10));
    }

    #[test]
    fn test_local_var_scope() {
        let local = LocalVarInfo::new("x".to_string(), "int".to_string(), -8).with_scope(10, 50);

        assert!(!local.is_in_scope(5));
        assert!(local.is_in_scope(10));
        assert!(local.is_in_scope(25));
        assert!(local.is_in_scope(49));
        assert!(!local.is_in_scope(50));
    }

    #[test]
    fn test_jit_debug_info() {
        let mut debug_info = JitDebugInfo::new();

        // Should not store when disabled
        debug_info.register(FunctionDebugInfo::new(
            "func1".to_string(),
            CompilationTier::BaselineCompiled,
        ));
        assert!(debug_info.get("func1").is_none());

        // Should store when enabled
        debug_info.enable();
        debug_info.register(FunctionDebugInfo::new(
            "func2".to_string(),
            CompilationTier::Optimized,
        ));
        assert!(debug_info.get("func2").is_some());
    }

    #[test]
    fn test_execution_tracer() {
        let mut tracer = ExecutionTracer::new();
        tracer.start();

        tracer.record("func1".to_string(), 0, TraceData::FunctionEntry);
        tracer.record("func1".to_string(), 10, TraceData::BlockEntry(1));
        tracer.record(
            "func1".to_string(),
            20,
            TraceData::FunctionExit(Some("42".to_string())),
        );

        assert_eq!(tracer.entries().len(), 3);
        tracer.stop();
    }

    #[test]
    fn test_code_dumper() {
        let dumper = CodeDumper::new();
        let code = vec![0x48, 0x89, 0xe5, 0x48, 0x83, 0xec, 0x10];

        let dump = dumper.dump(&code, 0x1000);
        assert!(dump.contains("1000"));
        assert!(dump.contains("48 89 e5"));
    }

    #[test]
    fn test_trace_data_display() {
        assert_eq!(format!("{}", TraceData::FunctionEntry), "ENTER");
        assert_eq!(
            format!("{}", TraceData::FunctionExit(Some("10".to_string()))),
            "EXIT -> 10"
        );
        assert_eq!(format!("{}", TraceData::BlockEntry(5)), "BLOCK 5");
    }
}
