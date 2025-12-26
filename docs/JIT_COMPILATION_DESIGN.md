# Tarqeem JIT Compilation Design

<div dir="rtl" align="right">

# ترقيم - تصميم الترجمة الفورية (JIT)

**الترجمة الآنية لتحسين الأداء مع الحفاظ على سرعة التطوير**

</div>

---

## Executive Summary

This document proposes a **tiered JIT (Just-In-Time) compilation** system for Tarqeem that combines the fast iteration of the interpreter with near-native execution speed. The design follows the proven approach of modern language runtimes (V8, HotSpot, LuaJIT) adapted to Tarqeem's architecture.

### Key Benefits

| Benefit | Description |
|---------|-------------|
| **Fast Startup** | Begin executing immediately via interpreter |
| **Optimized Hot Paths** | Compile frequently-executed code to native |
| **Reduced Latency** | No upfront compilation cost |
| **Adaptive Optimization** | Profile-guided compilation decisions |
| **REPL Performance** | Orders of magnitude faster for repeated code |

### Architecture Overview

```
Source Code
     ↓
  [Lexer → Parser → Semantic → IR]  (existing pipeline)
     ↓
┌────────────────────────────────────────────────────┐
│              JIT Execution Engine                   │
│  ┌──────────┐    ┌──────────┐    ┌──────────────┐  │
│  │  Tier 0  │ →  │  Tier 1  │ →  │   Tier 2     │  │
│  │Interpreter│    │Baseline  │    │ Optimizing   │  │
│  │ (cold)   │    │  JIT     │    │    JIT       │  │
│  └──────────┘    └──────────┘    └──────────────┘  │
│       ↑               ↑               ↑            │
│       └───────────────┴───────────────┘            │
│              Profiling & Tier-Up Decisions         │
└────────────────────────────────────────────────────┘
```

---

## 1. Current Architecture Analysis

### 1.1 Existing Pipeline

```rust
// Current execution paths in Tarqeem:

// Path A: Compilation (AOT)
Source → Lexer → Parser → Semantic → IR → Optimizer → LlvmCodegen → Binary

// Path B: Interpretation
Source → Lexer → Parser → Semantic → IR → Interpreter
```

### 1.2 Key Components for JIT Integration

| Component | Location | JIT Relevance |
|-----------|----------|---------------|
| `IrBuilder` | `src/ir/builder.rs` | Produces compilation units |
| `Module` | `src/ir/instruction.rs` | Contains functions/classes |
| `Function` | `src/ir/instruction.rs` | Primary JIT compilation unit |
| `Interpreter` | `src/interpreter/executor.rs` | Tier 0 execution |
| `LlvmCodegen` | `src/codegen/llvm/codegen.rs` | Text-based LLVM IR |
| `Optimizer` | `src/ir/opt/mod.rs` | IR optimization passes |

### 1.3 Current Limitations

1. **Text-based LLVM IR**: Cannot execute in-process
2. **No profiling**: No execution frequency tracking
3. **No compilation cache**: Each run rebuilds everything
4. **No tier-up mechanism**: Single execution mode

---

## 2. Tiered JIT Architecture

### 2.1 Execution Tiers

```
┌─────────────────────────────────────────────────────────────────┐
│                        EXECUTION TIERS                          │
├─────────────┬─────────────┬─────────────┬──────────────────────┤
│   Tier 0    │   Tier 1    │   Tier 2    │   Description        │
├─────────────┼─────────────┼─────────────┼──────────────────────┤
│ Interpreter │ Baseline    │ Optimizing  │                      │
│             │ JIT         │ JIT         │                      │
├─────────────┼─────────────┼─────────────┼──────────────────────┤
│ 0 ms        │ ~1-5 ms     │ ~10-50 ms   │ Compilation time     │
│ compile     │ per func    │ per func    │ per function         │
├─────────────┼─────────────┼─────────────┼──────────────────────┤
│ ~100x       │ ~5x         │ ~1x         │ Speed vs native      │
│ slower      │ slower      │ (native)    │                      │
├─────────────┼─────────────┼─────────────┼──────────────────────┤
│ Always      │ Hot (100+   │ Very hot    │ Trigger condition    │
│             │ calls)      │ (10K+ calls)│                      │
├─────────────┼─────────────┼─────────────┼──────────────────────┤
│ None        │ Basic       │ Full LLVM   │ Optimizations        │
│             │ (no inline) │ (inlining,  │                      │
│             │             │ vectorize)  │                      │
└─────────────┴─────────────┴─────────────┴──────────────────────┘
```

### 2.2 Tier Transitions

```
                    call_count >= 100
    ┌─────────┐    ─────────────────>    ┌─────────┐
    │ Tier 0  │                          │ Tier 1  │
    │Interpret│    <─────────────────    │Baseline │
    └─────────┘      deoptimization      │   JIT   │
                                         └─────────┘
                                              │
                         call_count >= 10,000 │
                                              ▼
                                         ┌─────────┐
                                         │ Tier 2  │
                                         │Optimize │
                                         │   JIT   │
                                         └─────────┘
```

---

## 3. Core Components Design

### 3.1 JIT Module Structure

```
src/
├── jit/                         # NEW: JIT compilation module
│   ├── mod.rs                   # Module exports
│   ├── profile.rs               # Profiling data collection
│   ├── compiler.rs              # JIT compilation orchestrator
│   ├── baseline/                # Tier 1: Baseline JIT
│   │   ├── mod.rs
│   │   └── codegen.rs           # Simple native codegen
│   ├── optimizing/              # Tier 2: Optimizing JIT
│   │   ├── mod.rs
│   │   ├── codegen.rs           # LLVM-based codegen
│   │   └── specialize.rs        # Type specialization
│   ├── runtime/                 # JIT runtime support
│   │   ├── mod.rs
│   │   ├── trampoline.rs        # Call trampolines
│   │   ├── osr.rs               # On-Stack Replacement
│   │   └── deopt.rs             # Deoptimization
│   └── cache.rs                 # Compiled code cache
```

### 3.2 Profile Data Structure

```rust
// src/jit/profile.rs

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Profile data for a single function
#[derive(Debug, Default)]
pub struct FunctionProfile {
    /// Number of times this function was called
    pub call_count: AtomicU64,

    /// Number of times this function was in a loop
    pub loop_iterations: AtomicU64,

    /// Observed argument types (for specialization)
    pub arg_types: Vec<ObservedType>,

    /// Branch taken/not-taken counts for each conditional
    pub branch_profiles: HashMap<u32, BranchProfile>,

    /// Current compilation tier
    pub tier: CompilationTier,

    /// Pointer to compiled code (if any)
    pub compiled_code: Option<CompiledFunction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTier {
    Interpreted,      // Tier 0
    BaselineCompiled, // Tier 1
    Optimized,        // Tier 2
}

#[derive(Debug, Clone)]
pub struct BranchProfile {
    pub taken_count: u64,
    pub not_taken_count: u64,
}

#[derive(Debug, Clone)]
pub enum ObservedType {
    AlwaysInt,
    AlwaysFloat,
    AlwaysString,
    AlwaysBool,
    AlwaysNull,
    AlwaysObject(String),  // Class name
    Mixed,                 // Multiple types observed
}

/// Global profiling state
#[derive(Debug, Default)]
pub struct ProfileData {
    pub functions: HashMap<String, Arc<FunctionProfile>>,
}

impl ProfileData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create(&mut self, func_name: &str) -> Arc<FunctionProfile> {
        self.functions
            .entry(func_name.to_string())
            .or_insert_with(|| Arc::new(FunctionProfile::default()))
            .clone()
    }

    pub fn should_tier_up(&self, func_name: &str, threshold: u64) -> bool {
        self.functions
            .get(func_name)
            .map(|p| p.call_count.load(Ordering::Relaxed) >= threshold)
            .unwrap_or(false)
    }
}
```

### 3.3 JIT Compiler Interface

```rust
// src/jit/compiler.rs

use crate::ir::{Function, Module};
use crate::jit::profile::{CompilationTier, FunctionProfile, ProfileData};
use std::sync::Arc;

/// Configuration for JIT compilation
#[derive(Debug, Clone)]
pub struct JitConfig {
    /// Threshold for Tier 0 → Tier 1 transition
    pub baseline_threshold: u64,

    /// Threshold for Tier 1 → Tier 2 transition
    pub optimizing_threshold: u64,

    /// Enable background compilation
    pub background_compilation: bool,

    /// Maximum functions to compile in background queue
    pub compile_queue_size: usize,

    /// Enable inline caching for method calls
    pub inline_caching: bool,

    /// Enable type specialization
    pub type_specialization: bool,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            baseline_threshold: 100,
            optimizing_threshold: 10_000,
            background_compilation: true,
            compile_queue_size: 64,
            inline_caching: true,
            type_specialization: true,
        }
    }
}

/// Compiled native function
pub struct CompiledFunction {
    /// Function name
    pub name: String,

    /// Native code pointer
    pub code_ptr: *const u8,

    /// Code size in bytes
    pub code_size: usize,

    /// Compilation tier
    pub tier: CompilationTier,

    /// Entry point for the function
    pub entry: extern "C" fn(*mut JitContext, *const Value) -> Value,
}

/// JIT execution context (passed to compiled functions)
#[repr(C)]
pub struct JitContext {
    /// Current module
    pub module: *const Module,

    /// Global variables
    pub globals: *mut HashMap<String, Value>,

    /// Profile data (for tier-up checks)
    pub profile: *mut ProfileData,

    /// Interpreter fallback
    pub interpreter: *mut Interpreter,
}

/// Main JIT compiler orchestrator
pub struct JitCompiler {
    config: JitConfig,
    profile_data: ProfileData,
    baseline_compiler: BaselineCompiler,
    optimizing_compiler: Option<OptimizingCompiler>,
    code_cache: CodeCache,
    compile_queue: CompileQueue,
}

impl JitCompiler {
    pub fn new(config: JitConfig) -> Self {
        Self {
            config: config.clone(),
            profile_data: ProfileData::new(),
            baseline_compiler: BaselineCompiler::new(),
            optimizing_compiler: if config.type_specialization {
                Some(OptimizingCompiler::new())
            } else {
                None
            },
            code_cache: CodeCache::new(),
            compile_queue: CompileQueue::new(config.compile_queue_size),
        }
    }

    /// Check if a function should be compiled
    pub fn should_compile(&self, func: &Function) -> Option<CompilationTier> {
        let profile = self.profile_data.functions.get(&func.name)?;
        let call_count = profile.call_count.load(Ordering::Relaxed);

        match profile.tier {
            CompilationTier::Interpreted if call_count >= self.config.baseline_threshold => {
                Some(CompilationTier::BaselineCompiled)
            }
            CompilationTier::BaselineCompiled if call_count >= self.config.optimizing_threshold => {
                Some(CompilationTier::Optimized)
            }
            _ => None,
        }
    }

    /// Compile a function at the specified tier
    pub fn compile(
        &mut self,
        module: &Module,
        func: &Function,
        tier: CompilationTier
    ) -> Result<CompiledFunction, JitError> {
        match tier {
            CompilationTier::Interpreted => {
                Err(JitError::CannotCompileInterpreted)
            }
            CompilationTier::BaselineCompiled => {
                self.baseline_compiler.compile(module, func)
            }
            CompilationTier::Optimized => {
                self.optimizing_compiler
                    .as_mut()
                    .ok_or(JitError::OptimizingDisabled)?
                    .compile(module, func, &self.profile_data)
            }
        }
    }

    /// Queue a function for background compilation
    pub fn queue_compile(&mut self, func_name: String, tier: CompilationTier) {
        if self.config.background_compilation {
            self.compile_queue.enqueue(func_name, tier);
        }
    }

    /// Process pending background compilations
    pub fn process_compile_queue(&mut self, module: &Module) {
        while let Some((func_name, tier)) = self.compile_queue.dequeue() {
            if let Some(func) = module.get_function_by_name(&func_name) {
                if let Ok(compiled) = self.compile(module, func, tier) {
                    self.code_cache.insert(func_name, compiled);
                }
            }
        }
    }

    /// Get compiled code for a function (if available)
    pub fn get_compiled(&self, func_name: &str) -> Option<&CompiledFunction> {
        self.code_cache.get(func_name)
    }

    /// Record a function call for profiling
    pub fn record_call(&mut self, func_name: &str) {
        let profile = self.profile_data.get_or_create(func_name);
        profile.call_count.fetch_add(1, Ordering::Relaxed);
    }
}
```

### 3.4 Baseline JIT Compiler (Tier 1)

```rust
// src/jit/baseline/codegen.rs

use crate::ir::{BasicBlock, Function, Instruction, Module};
use crate::jit::compiler::{CompiledFunction, JitError};
use crate::jit::profile::CompilationTier;

/// Simple, fast baseline compiler
///
/// Design goals:
/// - Compile quickly (< 5ms per function)
/// - Generate reasonably efficient code
/// - No complex optimizations
pub struct BaselineCompiler {
    /// Memory region for generated code
    code_buffer: CodeBuffer,
}

impl BaselineCompiler {
    pub fn new() -> Self {
        Self {
            code_buffer: CodeBuffer::new(1024 * 1024), // 1MB initial
        }
    }

    pub fn compile(
        &mut self,
        module: &Module,
        func: &Function
    ) -> Result<CompiledFunction, JitError> {
        let mut emitter = X64Emitter::new(&mut self.code_buffer);

        // Function prologue
        emitter.emit_prologue(func.params.len());

        // Allocate locals
        let locals_size = self.calculate_locals_size(func);
        emitter.emit_stack_alloc(locals_size);

        // Compile each basic block
        let mut block_offsets = Vec::new();
        for block in &func.blocks {
            block_offsets.push(emitter.current_offset());
            self.compile_block(&mut emitter, module, func, block)?;
        }

        // Patch jump targets
        emitter.patch_jumps(&block_offsets);

        // Function epilogue
        emitter.emit_epilogue();

        // Finalize and protect the code
        let (code_ptr, code_size) = emitter.finalize()?;

        Ok(CompiledFunction {
            name: func.name.clone(),
            code_ptr,
            code_size,
            tier: CompilationTier::BaselineCompiled,
            entry: unsafe { std::mem::transmute(code_ptr) },
        })
    }

    fn compile_block(
        &self,
        emitter: &mut X64Emitter,
        module: &Module,
        func: &Function,
        block: &BasicBlock,
    ) -> Result<(), JitError> {
        for inst in &block.instructions {
            self.compile_instruction(emitter, module, func, inst)?;
        }
        Ok(())
    }

    fn compile_instruction(
        &self,
        emitter: &mut X64Emitter,
        module: &Module,
        func: &Function,
        inst: &Instruction,
    ) -> Result<(), JitError> {
        match inst {
            Instruction::Const { dest, value, ty } => {
                emitter.emit_load_const(*dest, value)?;
            }

            Instruction::Binary { dest, op, left, right, ty } => {
                emitter.emit_binary(*dest, *op, *left, *right)?;
            }

            Instruction::Call { dest, func: func_id, args, ret_ty } => {
                // Check if callee is compiled
                // If yes: direct call
                // If no: call through interpreter trampoline
                emitter.emit_call(*dest, func_id, args)?;
            }

            Instruction::Return { value } => {
                emitter.emit_return(*value)?;
            }

            Instruction::Branch { cond, then_block, else_block } => {
                emitter.emit_branch(*cond, *then_block, *else_block)?;
            }

            Instruction::Jump { target } => {
                emitter.emit_jump(*target)?;
            }

            // ... other instructions

            _ => {
                // For unsupported instructions, emit a call to interpreter
                emitter.emit_interpreter_fallback(inst)?;
            }
        }
        Ok(())
    }

    fn calculate_locals_size(&self, func: &Function) -> usize {
        // Count unique VarIds and multiply by 8 bytes each
        let max_var = func.blocks.iter()
            .flat_map(|b| &b.instructions)
            .filter_map(|i| i.dest_var())
            .map(|v| v.0)
            .max()
            .unwrap_or(0);

        ((max_var as usize + 1) * 8 + 15) & !15 // Align to 16 bytes
    }
}
```

### 3.5 Optimizing JIT Compiler (Tier 2)

```rust
// src/jit/optimizing/codegen.rs

use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Module as LlvmModule;
use inkwell::OptimizationLevel;

use crate::ir::{Function, Module};
use crate::jit::compiler::{CompiledFunction, JitError};
use crate::jit::profile::{CompilationTier, FunctionProfile, ProfileData};

/// Optimizing compiler using LLVM
///
/// Design goals:
/// - Generate optimal native code
/// - Use profile data for guided optimization
/// - Support type specialization
pub struct OptimizingCompiler {
    llvm_context: Context,
}

impl OptimizingCompiler {
    pub fn new() -> Self {
        Self {
            llvm_context: Context::create(),
        }
    }

    pub fn compile(
        &mut self,
        module: &Module,
        func: &Function,
        profile: &ProfileData,
    ) -> Result<CompiledFunction, JitError> {
        // Create LLVM module for this function
        let llvm_module = self.llvm_context.create_module(&func.name);

        // Build LLVM IR with profile-guided optimizations
        let builder = self.llvm_context.create_builder();

        // Get profile for this function
        let func_profile = profile.functions.get(&func.name);

        // Emit LLVM IR
        self.emit_function(&llvm_module, &builder, func, func_profile)?;

        // Create execution engine with optimizations
        let execution_engine = llvm_module
            .create_jit_execution_engine(OptimizationLevel::Aggressive)
            .map_err(|e| JitError::LlvmError(e.to_string()))?;

        // Run optimization passes
        self.run_optimization_passes(&llvm_module, func_profile);

        // Get function pointer
        let func_ptr = execution_engine
            .get_function_address(&func.name)
            .map_err(|e| JitError::LlvmError(e.to_string()))?;

        Ok(CompiledFunction {
            name: func.name.clone(),
            code_ptr: func_ptr as *const u8,
            code_size: 0, // LLVM manages this
            tier: CompilationTier::Optimized,
            entry: unsafe { std::mem::transmute(func_ptr) },
        })
    }

    fn emit_function(
        &self,
        llvm_module: &LlvmModule,
        builder: &inkwell::builder::Builder,
        func: &Function,
        profile: Option<&std::sync::Arc<FunctionProfile>>,
    ) -> Result<(), JitError> {
        // Similar to current LlvmCodegen, but:
        // 1. Uses inkwell API instead of text
        // 2. Adds profile-guided metadata
        // 3. Specializes based on observed types

        // ... LLVM IR generation code ...

        Ok(())
    }

    fn run_optimization_passes(
        &self,
        llvm_module: &LlvmModule,
        profile: Option<&std::sync::Arc<FunctionProfile>>,
    ) {
        // Use PassManager to run LLVM optimization passes
        // Adjust passes based on profile data

        let pass_manager = inkwell::passes::PassManager::create(());

        // Always run these
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();

        // Run if function is very hot
        if profile.map(|p| p.call_count.load(std::sync::atomic::Ordering::Relaxed) > 100_000).unwrap_or(false) {
            pass_manager.add_loop_vectorize_pass();
            pass_manager.add_slp_vectorize_pass();
        }

        pass_manager.run_on(llvm_module);
    }
}
```

### 3.6 JIT-Enabled Interpreter

```rust
// src/jit/executor.rs

use crate::interpreter::{Interpreter, RuntimeResult, Value};
use crate::ir::{FuncId, Module};
use crate::jit::compiler::{JitCompiler, JitConfig, JitContext};
use crate::jit::profile::CompilationTier;

/// JIT-enabled executor that combines interpretation and compilation
pub struct JitExecutor {
    module: Module,
    interpreter: Interpreter,
    jit_compiler: JitCompiler,
    jit_context: JitContext,
}

impl JitExecutor {
    pub fn new(module: Module, config: JitConfig) -> Self {
        let interpreter = Interpreter::new(module.clone());
        let jit_compiler = JitCompiler::new(config);

        Self {
            module,
            interpreter,
            jit_compiler,
            jit_context: JitContext::default(),
        }
    }

    pub fn run(&mut self) -> RuntimeResult<Value> {
        // Initialize globals
        self.interpreter.init_globals()?;

        // Find and call main function
        let main_func = self.find_main_function()?;
        self.call_function(&main_func, vec![])
    }

    pub fn call_function(&mut self, func_id: &FuncId, args: Vec<Value>) -> RuntimeResult<Value> {
        // Record the call for profiling
        self.jit_compiler.record_call(&func_id.0);

        // Check if we have compiled code
        if let Some(compiled) = self.jit_compiler.get_compiled(&func_id.0) {
            // Execute compiled code
            return self.execute_compiled(compiled, args);
        }

        // Check if we should compile
        if let Some(tier) = self.should_compile(&func_id.0) {
            // Queue for background compilation
            self.jit_compiler.queue_compile(func_id.0.clone(), tier);
        }

        // Fall back to interpreter
        self.interpreter.call_function(func_id, args)
    }

    fn should_compile(&self, func_name: &str) -> Option<CompilationTier> {
        self.module
            .get_function_by_name(func_name)
            .and_then(|func| self.jit_compiler.should_compile(func))
    }

    fn execute_compiled(&mut self, compiled: &CompiledFunction, args: Vec<Value>) -> RuntimeResult<Value> {
        // Prepare arguments
        let args_ptr = args.as_ptr();

        // Call the compiled function
        let result = unsafe {
            (compiled.entry)(&mut self.jit_context as *mut _, args_ptr)
        };

        Ok(result)
    }

    fn find_main_function(&self) -> RuntimeResult<FuncId> {
        // Same as interpreter
        self.interpreter.find_main_function()
    }

    /// Process any pending background compilations
    pub fn poll_compilations(&mut self) {
        self.jit_compiler.process_compile_queue(&self.module);
    }
}
```

---

## 4. On-Stack Replacement (OSR)

OSR allows switching from interpreted to compiled code mid-execution, essential for long-running loops.

```rust
// src/jit/runtime/osr.rs

use crate::ir::{BlockId, Function, VarId};
use crate::jit::compiler::CompiledFunction;
use crate::interpreter::Value;

/// OSR entry point for a loop
#[derive(Debug)]
pub struct OsrEntry {
    /// Block ID where OSR can occur
    pub block_id: BlockId,

    /// Offset into compiled code for this entry
    pub native_offset: usize,

    /// Mapping from IR variable to native stack slot
    pub var_mapping: Vec<(VarId, i32)>,
}

/// Prepare OSR transition
pub fn prepare_osr(
    func: &Function,
    compiled: &CompiledFunction,
    current_block: BlockId,
    locals: &HashMap<VarId, Value>,
) -> Option<OsrTransition> {
    // Find OSR entry point
    let entry = compiled.osr_entries.iter()
        .find(|e| e.block_id == current_block)?;

    // Prepare native stack frame
    let mut native_frame = vec![0u8; compiled.frame_size];

    // Copy locals to native frame
    for (var_id, stack_offset) in &entry.var_mapping {
        if let Some(value) = locals.get(var_id) {
            let bytes = value.to_native_bytes();
            let offset = *stack_offset as usize;
            native_frame[offset..offset + bytes.len()].copy_from_slice(&bytes);
        }
    }

    Some(OsrTransition {
        entry_point: unsafe { compiled.code_ptr.add(entry.native_offset) },
        native_frame,
    })
}

pub struct OsrTransition {
    pub entry_point: *const u8,
    pub native_frame: Vec<u8>,
}
```

---

## 5. CLI Integration

### 5.1 New CLI Options

```rust
// Additions to src/cli/commands.rs

#[derive(Debug, Clone, clap::Args)]
pub struct JitOptions {
    /// Enable JIT compilation
    #[arg(long, default_value = "true")]
    pub jit: bool,

    /// Tier-1 compilation threshold (function call count)
    #[arg(long, default_value = "100")]
    pub baseline_threshold: u64,

    /// Tier-2 compilation threshold
    #[arg(long, default_value = "10000")]
    pub optimize_threshold: u64,

    /// Enable background compilation
    #[arg(long, default_value = "true")]
    pub background_compile: bool,

    /// Print JIT compilation events
    #[arg(long)]
    pub jit_verbose: bool,

    /// Dump generated native code
    #[arg(long)]
    pub dump_jit_code: bool,
}

// New run command with JIT
Commands::Run { file, jit_opts } => {
    // ... parse, analyze, build IR ...

    if jit_opts.jit {
        let config = JitConfig {
            baseline_threshold: jit_opts.baseline_threshold,
            optimizing_threshold: jit_opts.optimize_threshold,
            background_compilation: jit_opts.background_compile,
            ..Default::default()
        };

        let mut executor = JitExecutor::new(ir_module, config);
        if jit_opts.jit_verbose {
            executor.set_verbose(true);
        }
        executor.run()?;
    } else {
        // Pure interpreter mode
        let mut interpreter = Interpreter::new(ir_module);
        interpreter.run()?;
    }
}
```

### 5.2 Example Usage

```bash
# Run with default JIT settings
tarqeem run program.trq

# Run with custom thresholds
tarqeem run program.trq --baseline-threshold=50 --optimize-threshold=5000

# Disable JIT (pure interpretation)
tarqeem run program.trq --jit=false

# Debug JIT behavior
tarqeem run program.trq --jit-verbose

# Dump generated native code
tarqeem run program.trq --dump-jit-code > jit_output.asm
```

---

## 6. Usage Examples in Tarqeem

### 6.1 Automatic JIT Optimization

```tarqeem
// برنامج مُحسَّن تلقائياً بالترجمة الفورية
// Automatically JIT-optimized program

دالة فيبوناتشي(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع ن
    }
    أرجع فيبوناتشي(ن - 1) + فيبوناتشي(ن - 2)
}

// Main function calls فيبوناتشي many times
// After ~100 calls, فيبوناتشي gets baseline-compiled
// After ~10,000 calls, it gets fully optimized

لكل (متغير ع = 0؛ ع < 40؛ ع++) {
    اطبع(فيبوناتشي(ع))
}
```

**Execution flow:**
1. First calls: Interpreted (slow, ~100x native)
2. After 100 calls: Baseline compiled (fast, ~5x native)
3. After 10K calls: Fully optimized (near native speed)

### 6.2 Loop Optimization with OSR

```tarqeem
// الترجمة الفورية للحلقات الطويلة
// JIT compilation for long-running loops

دالة حساب_معقد() {
    متغير مجموع: عدد_عشري = 0.0

    // هذه الحلقة ستُترجَم فورياً بعد 10,000 تكرار
    // This loop will be JIT-compiled after 10,000 iterations
    لكل (متغير ع = 0؛ ع < 1000000؛ ع++) {
        مجموع = مجموع + (ع * 0.001)
    }

    اطبع(مجموع)
}

حساب_معقد()
```

**What happens:**
1. Loop starts in interpreter
2. After 10,000 iterations, OSR triggers
3. Function is compiled to native code
4. Execution continues at native speed
5. Remaining 990,000 iterations run at native speed

### 6.3 Method Call Optimization

```tarqeem
// تحسين استدعاءات الدوال مع التخزين المؤقت
// Method call optimization with inline caching

ميثاق قابل_للحساب {
    دالة احسب() -> عدد
}

صنف مجموع يلتزم قابل_للحساب {
    خاص القيم: مصفوفة<عدد>

    عام دالة احسب() -> عدد {
        متغير م = 0
        لكل ق في هذا.القيم {
            م = م + ق
        }
        أرجع م
    }
}

صنف جداء يلتزم قابل_للحساب {
    خاص القيم: مصفوفة<عدد>

    عام دالة احسب() -> عدد {
        متغير م = 1
        لكل ق في هذا.القيم {
            م = م * ق
        }
        أرجع م
    }
}

// الترجمة الفورية تُخزِّن نوع الكائن مؤقتاً
// JIT caches the object type for fast dispatch
دالة معالجة(عناصر: مصفوفة<قابل_للحساب>) {
    لكل عنصر في عناصر {
        اطبع(عنصر.احسب())  // Inline-cached after first call
    }
}
```

### 6.4 Type-Specialized Functions

```tarqeem
// التخصص بناءً على الأنماط المُلاحَظة
// Type specialization based on observed types

دالة معممة<ن>(قيمة: ن) -> ن {
    // JIT observes that قيمة is always عدد
    // Generates specialized code for عدد
    أرجع قيمة
}

// If always called with عدد, JIT generates:
// fn معممة_عدد(قيمة: i64) -> i64

متغير نتيجة1 = معممة(42)     // Triggers specialization
متغير نتيجة2 = معممة(100)    // Uses specialized version
متغير نتيجة3 = معممة(999)    // Uses specialized version
```

### 6.5 REPL with JIT

```tarqeem
// في وضع REPL، تُحفظ الدوال المُترجَمة بين الأسطر
// In REPL mode, compiled functions are cached between lines

>> دالة ضعف(س: عدد) -> عدد { أرجع س * 2 }
تم تعريف الدالة: ضعف

>> لكل (متغير ع = 0؛ ع < 1000؛ ع++) { ضعف(ع) }
[الترجمة الفورية: ضعف مُترجَمة إلى Tier 1]
منتهي

>> ضعف(5)  // Uses cached compiled version
10
```

---

## 7. Performance Expectations

### 7.1 Benchmark Targets

| Workload | Interpreter | Tier 1 | Tier 2 | Native (AOT) |
|----------|-------------|--------|--------|--------------|
| Fibonacci(35) | 8,000 ms | 1,500 ms | 100 ms | 80 ms |
| Array sum (1M) | 500 ms | 80 ms | 8 ms | 6 ms |
| String ops | 200 ms | 50 ms | 20 ms | 15 ms |
| Method calls | 1,000 ms | 150 ms | 40 ms | 30 ms |

### 7.2 Memory Overhead

| Component | Memory Usage |
|-----------|-------------|
| Base interpreter | ~5 MB |
| Profile data | ~100 KB per 1000 functions |
| Baseline JIT code | ~1-5 KB per function |
| Optimizing JIT code | ~2-10 KB per function |
| Code cache | ~10-50 MB max |

---

## 8. Implementation Roadmap

### Phase 1: Foundation (v2.0)
- [ ] Add `inkwell` dependency
- [ ] Create `src/jit/` module structure
- [ ] Implement `ProfileData` and profiling hooks
- [ ] Add `JitConfig` and CLI options

### Phase 2: Baseline JIT (v2.1)
- [ ] Implement `BaselineCompiler`
- [ ] Create x86-64 code emitter
- [ ] Add function call trampolines
- [ ] Integrate with interpreter

### Phase 3: Optimizing JIT (v2.2)
- [ ] Implement `OptimizingCompiler` using inkwell
- [ ] Add profile-guided optimization
- [ ] Implement type specialization
- [ ] Add inline caching

### Phase 4: Advanced Features (v2.3)
- [ ] On-Stack Replacement (OSR)
- [ ] Background compilation
- [ ] Deoptimization support
- [ ] Code cache management

### Phase 5: Polish (v2.4)
- [ ] Performance tuning
- [ ] Memory optimization
- [ ] Debugging support
- [ ] Documentation

---

## 9. Dependencies

### New Cargo Dependencies

```toml
[dependencies]
# JIT compilation
inkwell = { version = "0.5", features = ["llvm18-0"] }

# Code generation
cranelift-codegen = "0.110"  # Alternative to inkwell for Tier 1
cranelift-frontend = "0.110"
cranelift-jit = "0.110"

# Memory management
memmap2 = "0.9"      # Executable memory mapping
region = "3.0"       # Memory protection

# Profiling
parking_lot = "0.12" # Fast locks for profile data

[features]
jit = ["inkwell", "memmap2", "region"]
cranelift-baseline = ["cranelift-codegen", "cranelift-frontend", "cranelift-jit"]
```

---

## 10. Alternative Approaches

### 10.1 Cranelift-Only JIT

Instead of LLVM for Tier 2, use Cranelift for both tiers:

**Pros:**
- Faster compilation
- No LLVM dependency (smaller binary)
- Simpler integration

**Cons:**
- Less optimization than LLVM
- ~20% slower generated code

### 10.2 Tracing JIT

Instead of method-based JIT, use tracing:

**Pros:**
- Better for dynamic code
- Cross-function optimization
- Simpler OSR

**Cons:**
- More complex implementation
- Higher memory usage
- Harder to debug

### 10.3 Copy-and-Patch

Instead of full code generation, use pre-compiled templates:

**Pros:**
- Very fast compilation (~1ms)
- Simple implementation
- No LLVM/Cranelift needed

**Cons:**
- Limited optimization
- More complex maintenance
- Template generation needed

---

## 11. Conclusion

This JIT design provides a solid foundation for significantly improving Tarqeem's runtime performance while maintaining the fast iteration experience of the interpreter. The tiered approach allows:

1. **Immediate execution**: No upfront compilation cost
2. **Gradual optimization**: Hot code gets progressively faster
3. **Adaptive behavior**: Profile-guided optimization decisions
4. **Graceful degradation**: Always falls back to interpreter

The implementation can proceed incrementally, with each phase delivering measurable benefits. The baseline JIT alone (Phase 2) would improve performance by 10-20x for hot functions, making it a high-value milestone.

---

**المؤلف / Author:** Claude
**التاريخ / Date:** 2025-12-25
**الإصدار / Version:** 1.0.0
