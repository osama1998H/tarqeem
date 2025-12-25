# WebAssembly Compilation Design for Tarqeem

## Overview

This document describes the comprehensive design for adding WebAssembly (WASM) compilation support to Tarqeem, enabling Arabic code to run in web browsers and other WASM environments.

## Architecture

### Compilation Pipeline for WASM

```
Source (.trq)
     │
     ▼
┌──────────────────┐
│  Lexer/Parser    │  (unchanged)
└──────────────────┘
     │
     ▼
┌──────────────────┐
│ Semantic Analyzer│  (unchanged)
└──────────────────┘
     │
     ▼
┌──────────────────┐
│   IR Builder     │  (unchanged)
└──────────────────┘
     │
     ▼
┌──────────────────┐
│   Optimizer      │  (unchanged)
└──────────────────┘
     │
     ├─────────────────────┬───────────────────┐
     ▼                     ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌─────────────────┐
│ LLVM Codegen │    │ LLVM Codegen │    │ LLVM Codegen    │
│   (Native)   │    │  (WASM32)    │    │  (WASI-p1)      │
└──────────────┘    └──────────────┘    └─────────────────┘
     │                     │                   │
     ▼                     ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌─────────────────┐
│    Linker    │    │  WASM Linker │    │   WASI Linker   │
│   (clang)    │    │  (wasm-ld)   │    │   (wasm-ld)     │
└──────────────┘    └──────────────┘    └─────────────────┘
     │                     │                   │
     ▼                     ▼                   ▼
   Native              .wasm +              .wasm
  Executable         JS Bindings         (standalone)
```

## Target Triples

### Supported WASM Targets

| Triple | Description | Use Case |
|--------|-------------|----------|
| `wasm32-unknown-unknown` | Pure WASM | Browser via JS bindings |
| `wasm32-wasip1` | WASM with WASI | Standalone CLI, Node.js |
| `wasm32-wasip2` | WASM Component Model | Future component support |

### Data Layout (WASM32)

```
e-m:e-p:32:32-i64:64-n32:64-S128-ni:1:10:20
```

- Little endian (`e`)
- Mangling: ELF style (`m:e`)
- Pointer: 32-bit (`p:32:32`)
- Native integers: 32 and 64 bit (`n32:64`)
- Stack alignment: 128 bits (`S128`)

## Implementation Components

### 1. Target Configuration (`src/codegen/target.rs`)

```rust
impl DataLayout {
    pub fn wasm32() -> Self {
        Self {
            endianness: 'e',
            pointer_bits: 32,
            stack_alignment: 128,
            native_integers: vec![32, 64],
        }
    }

    pub fn to_llvm_string_wasm(&self) -> String {
        "e-m:e-p:32:32-i64:64-n32:64-S128-ni:1:10:20".to_string()
    }
}

impl TargetTriple {
    pub fn wasm32_unknown() -> Self {
        Self::new("wasm32", "unknown", "unknown", None)
    }

    pub fn wasm32_wasi() -> Self {
        Self::new("wasm32", "wasi", "wasi", None)
    }

    pub fn is_wasm(&self) -> bool {
        self.arch.starts_with("wasm")
    }
}
```

### 2. Linker Extension (`src/codegen/linker.rs`)

```rust
impl Linker {
    pub fn compile_to_wasm(
        &self,
        llvm_ir: &str,
        output: &Path,
        export_functions: &[&str],
    ) -> Result<(), LinkerError> {
        // 1. Write LLVM IR to .ll file
        // 2. Compile with clang --target=wasm32
        // 3. Link with wasm-ld
        // 4. Generate JS bindings if requested
    }
}
```

### 3. CLI Integration (`src/cli/mod.rs`)

```rust
Commands::Compile {
    // ... existing flags ...

    /// Emit WebAssembly binary
    #[arg(long)]
    emit_wasm: bool,

    /// Generate JavaScript bindings for WASM
    #[arg(long)]
    wasm_js_bindings: bool,

    /// Export all public functions to WASM
    #[arg(long)]
    wasm_export_all: bool,

    /// WASM memory pages (64KB each)
    #[arg(long, default_value = "16")]
    wasm_memory_pages: u32,

    /// Use WASI for I/O instead of JS bindings
    #[arg(long)]
    wasi: bool,
}
```

### 4. EmitType Extension

```rust
pub enum EmitType {
    LlvmIr,
    Assembly,
    Object,
    Executable,
    Wasm,        // NEW: .wasm output
    WasmModule,  // NEW: .wasm + .js bindings
}

impl EmitType {
    pub fn extension(&self) -> &'static str {
        match self {
            EmitType::Wasm => "wasm",
            EmitType::WasmModule => "wasm",
            // ... existing cases
        }
    }
}
```

## WASM Runtime Library

### Structure

```
runtime/
├── ... (existing files)
└── wasm/
    ├── runtime_wasm.c     # WASM-compatible runtime
    ├── memory_wasm.c      # Linear memory allocator
    ├── string_wasm.c      # String operations
    ├── array_wasm.c       # Array operations
    ├── io_wasm.c          # I/O stubs (calls JS)
    ├── imports.js         # JavaScript imports
    └── Makefile.wasm      # WASM build config
```

### Memory Management for WASM

WASM uses linear memory, so we need a custom allocator:

```c
// runtime/wasm/memory_wasm.c

// Memory layout:
// [0 - 1023]: Reserved (null pointer trap)
// [1024 - heap_start]: Static data
// [heap_start - memory_end]: Heap (bump allocator + free list)

static uint32_t heap_ptr = 1024;  // Start of heap

void* trq_alloc(int32_t size) {
    // Align to 8 bytes
    size = (size + 7) & ~7;

    // Add refcount header (4 bytes for wasm32)
    size += 4;

    uint32_t addr = heap_ptr;
    heap_ptr += size;

    // Initialize refcount to 1
    *((int32_t*)addr) = 1;

    return (void*)(addr + 4);
}
```

### I/O Through JavaScript Imports

For browser WASM, I/O must go through JavaScript:

```c
// runtime/wasm/io_wasm.c

// These are provided by JavaScript
extern void __trq_js_print(const char* ptr, int32_t len);
extern int32_t __trq_js_input(char* buf, int32_t max_len);

void trq_print(TrqString* str) {
    __trq_js_print(str->data, str->len);
}
```

### JavaScript Bindings

```javascript
// tarqeem_runtime.js

const TarqeemRuntime = {
    memory: null,
    instance: null,

    async load(wasmPath) {
        const imports = {
            env: {
                __trq_js_print: (ptr, len) => {
                    const view = new Uint8Array(this.memory.buffer, ptr, len);
                    const text = new TextDecoder('utf-8').decode(view);
                    console.log(text);
                },
                __trq_js_input: (buf, maxLen) => {
                    // Use prompt() in browser or readline in Node.js
                    const input = prompt('إدخال:') || '';
                    const bytes = new TextEncoder().encode(input);
                    const view = new Uint8Array(this.memory.buffer, buf, maxLen);
                    view.set(bytes.slice(0, maxLen));
                    return Math.min(bytes.length, maxLen);
                },
                memory: new WebAssembly.Memory({ initial: 16 })
            }
        };

        const response = await fetch(wasmPath);
        const bytes = await response.arrayBuffer();
        const module = await WebAssembly.compile(bytes);
        this.instance = await WebAssembly.instantiate(module, imports);
        this.memory = imports.env.memory;

        return this;
    },

    // Call exported Tarqeem functions
    call(funcName, ...args) {
        return this.instance.exports[funcName](...args);
    },

    // Run the main function
    run() {
        if (this.instance.exports.__main__) {
            return this.instance.exports.__main__();
        }
        if (this.instance.exports.main) {
            return this.instance.exports.main();
        }
        throw new Error('No main function found');
    }
};

export default TarqeemRuntime;
```

## Usage in Tarqeem

### Basic WASM Compilation

```bash
# Compile to WASM (browser-ready)
tarqeem compile برنامج.trq --emit-wasm -o برنامج.wasm

# Compile with JS bindings
tarqeem compile برنامج.trq --emit-wasm --wasm-js-bindings -o برنامج

# This produces:
#   برنامج.wasm     - WebAssembly binary
#   برنامج.js       - JavaScript loader/bindings

# Compile for WASI (standalone)
tarqeem compile برنامج.trq --target wasm32-wasip1 -o برنامج.wasm
```

### Export Functions for JavaScript

```tarqeem
// حاسبة.trq

// Use صدّر to export functions to WASM
صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

صدّر دالة ضرب(أ: عدد، ب: عدد) -> عدد {
    أرجع أ * ب
}

صدّر دالة قوة(أساس: عدد، أس: عدد) -> عدد {
    متغير نتيجة = 1
    لكل (متغير ع = 0؛ ع < أس؛ ع++) {
        نتيجة = نتيجة * أساس
    }
    أرجع نتيجة
}
```

Compile and use:

```bash
tarqeem compile حاسبة.trq --emit-wasm --wasm-export-all -o حاسبة
```

```html
<!-- index.html -->
<!DOCTYPE html>
<html dir="rtl" lang="ar">
<head>
    <meta charset="UTF-8">
    <title>حاسبة ترقيم</title>
</head>
<body>
    <h1>حاسبة ترقيم</h1>
    <input type="number" id="أ" placeholder="الرقم الأول">
    <input type="number" id="ب" placeholder="الرقم الثاني">
    <button onclick="compute('جمع')">جمع</button>
    <button onclick="compute('ضرب')">ضرب</button>
    <p id="نتيجة"></p>

    <script type="module">
        import TarqeemRuntime from './tarqeem_runtime.js';

        const runtime = await TarqeemRuntime.load('./حاسبة.wasm');

        window.compute = (op) => {
            const أ = parseInt(document.getElementById('أ').value);
            const ب = parseInt(document.getElementById('ب').value);

            let نتيجة;
            if (op === 'جمع') {
                نتيجة = runtime.call('جمع', أ, ب);
            } else {
                نتيجة = runtime.call('ضرب', أ, ب);
            }

            document.getElementById('نتيجة').textContent = `النتيجة: ${نتيجة}`;
        };
    </script>
</body>
</html>
```

### Interactive Program with I/O

```tarqeem
// تحية.trq

صدّر دالة __main__() {
    اطبع("مرحباً بك في برنامج ترقيم!")
    اطبع("ما اسمك؟")

    متغير اسم = ادخل()
    اطبع("أهلاً يا " + اسم + "!")
}
```

```bash
tarqeem compile تحية.trq --emit-wasm --wasm-js-bindings -o تحية
```

```html
<!-- تحية.html -->
<!DOCTYPE html>
<html dir="rtl" lang="ar">
<head>
    <meta charset="UTF-8">
    <title>تحية</title>
</head>
<body>
    <div id="terminal" style="font-family: monospace; white-space: pre;"></div>
    <input type="text" id="input" style="display: none;">

    <script type="module">
        import TarqeemRuntime from './tarqeem_runtime.js';

        const terminal = document.getElementById('terminal');
        const input = document.getElementById('input');

        // Override print to write to terminal
        TarqeemRuntime.overrides = {
            print: (text) => {
                terminal.textContent += text + '\n';
            },
            input: async () => {
                input.style.display = 'block';
                input.focus();
                return new Promise(resolve => {
                    input.onkeydown = (e) => {
                        if (e.key === 'Enter') {
                            const value = input.value;
                            input.value = '';
                            input.style.display = 'none';
                            terminal.textContent += value + '\n';
                            resolve(value);
                        }
                    };
                });
            }
        };

        const runtime = await TarqeemRuntime.load('./تحية.wasm');
        runtime.run();
    </script>
</body>
</html>
```

### WASI for Command-Line Usage

```tarqeem
// سطر_أوامر.trq

صدّر دالة __main__() {
    اطبع("برنامج يعمل بـ WASI!")

    // File operations work with WASI
    متغير محتوى = اقرأ_ملف("بيانات.txt")
    اطبع("محتوى الملف: " + محتوى)
}
```

```bash
# Compile for WASI
tarqeem compile سطر_أوامر.trq --target wasm32-wasip1 -o برنامج.wasm

# Run with wasmtime
wasmtime برنامج.wasm

# Run with Node.js
node --experimental-wasi-unstable-preview1 run_wasi.js برنامج.wasm
```

## Type Mapping for WASM

| Tarqeem Type | IR Type | WASM Type | Notes |
|--------------|---------|-----------|-------|
| `عدد` (int) | i64 | i64 | Native 64-bit |
| `عدد_عشري` (float) | f64 | f64 | Native |
| `منطقي` (bool) | i1 | i32 | Promoted to 32-bit |
| `نص` (string) | ptr | i32 | Pointer in linear memory |
| `مصفوفة` (array) | ptr | i32 | Pointer in linear memory |
| Objects | ptr | i32 | Pointer in linear memory |

## Special Considerations

### 1. No Global Mutable State Exports

WASM globals cannot be directly exported as mutable. Global variables are stored in linear memory.

### 2. Memory Growth

The runtime can request memory growth using `memory.grow`:

```c
bool grow_memory(int32_t pages) {
    int32_t old_size = __builtin_wasm_memory_grow(0, pages);
    return old_size != -1;
}
```

### 3. Stack Size

Default stack size is 64KB. Can be configured:

```bash
tarqeem compile برنامج.trq --emit-wasm --wasm-stack-size 128
```

### 4. String Encoding

All strings in WASM remain UTF-8 encoded. The JavaScript bindings handle conversion to/from JavaScript strings.

### 5. Exception Handling

WASM exception handling is still evolving. Current approach:
- Use a global exception flag
- `throw` sets the flag and returns immediately
- `try/catch` checks the flag after calls

## Build Requirements

### Dependencies

- LLVM 14+ with WebAssembly target enabled
- `wasm-ld` (part of LLVM)
- Optional: `wasm-opt` from Binaryen for optimization

### Checking WASM Support

```bash
# Check LLVM WASM target
llc --version | grep wasm

# Check wasm-ld
wasm-ld --version
```

## Implementation Phases

### Phase 1: Basic WASM Output ✅ Complete
- [x] Add wasm32 target configuration
- [x] Extend linker for WASM output
- [x] Add `--emit-wasm` CLI flag
- [x] Generate minimal WASM without runtime

### Phase 2: WASM Runtime ✅ Complete
- [x] Create WASM-compatible allocator
- [x] Port string operations
- [x] Port array operations
- [x] Implement I/O stubs

### Phase 3: JavaScript Bindings ✅ Complete
- [x] Generate JS loader code
- [x] Handle async I/O in browser
- [x] Export function discovery
- [x] Type conversion helpers

### Phase 4: WASI Support 🔄 Partial
- [x] Add wasm32-wasip1 target
- [ ] Link with WASI libc (planned)
- [ ] Enable file system access (planned)
- [ ] Enable networking (planned)

### Phase 5: Optimization ⏳ Future
- [ ] Integrate wasm-opt
- [ ] Dead code elimination for WASM
- [ ] Memory layout optimization
- [ ] Size reduction techniques

## CLI Reference

```bash
# Basic WASM compilation
tarqeem compile <file>.trq --emit-wasm -o <output>.wasm

# With optimizations
tarqeem compile <file>.trq --emit-wasm -O2 -o <output>.wasm

# With JS bindings
tarqeem compile <file>.trq --emit-wasm --wasm-js-bindings -o <output>

# For WASI
tarqeem compile <file>.trq --target wasm32-wasip1 -o <output>.wasm

# Export all public functions
tarqeem compile <file>.trq --emit-wasm --wasm-export-all -o <output>.wasm

# Custom memory size (in 64KB pages)
tarqeem compile <file>.trq --emit-wasm --wasm-memory-pages 32 -o <output>.wasm

# Generate source map (for debugging)
tarqeem compile <file>.trq --emit-wasm -g --wasm-source-map -o <output>.wasm
```

## Error Messages

Bilingual error messages for WASM-specific errors:

| Error | English | Arabic |
|-------|---------|--------|
| No WASM support | "LLVM not configured with WebAssembly target" | "LLVM غير مهيأ لدعم WebAssembly" |
| wasm-ld not found | "wasm-ld not found. Install LLVM with WebAssembly support" | "لم يُعثر على wasm-ld. ثبّت LLVM مع دعم WebAssembly" |
| Memory limit | "WASM memory limit exceeded" | "تم تجاوز حد ذاكرة WebAssembly" |
| Export error | "Cannot export function '{}' to WASM" | "لا يمكن تصدير الدالة '{}' إلى WebAssembly" |

## Future Enhancements

1. **WASM Threads**: Support for `SharedArrayBuffer` and atomics
2. **WASM SIMD**: Vector operations for math-heavy code
3. **Component Model**: WASM components for better interop
4. **Streaming Compilation**: Compile while downloading
5. **Debug Support**: Source-level debugging in browser DevTools
