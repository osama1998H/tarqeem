# Profiling Guide for Tarqeem Compiler

This guide describes how to profile the Tarqeem compiler to identify performance bottlenecks and measure optimization effectiveness.

## Prerequisites

Install the required profiling tools:

```bash
# Linux
sudo apt-get install linux-perf
cargo install flamegraph

# macOS
brew install instruments
cargo install flamegraph
```

## Running Benchmarks

### Quick Benchmark Run

Run all benchmarks:

```bash
cargo bench
```

Run specific benchmark suites:

```bash
# Lexer throughput
cargo bench --bench lexer

# Parser speed
cargo bench --bench parser

# Semantic analysis
cargo bench --bench semantic

# IR generation
cargo bench --bench ir_generation

# Optimizer passes
cargo bench --bench optimizer

# Full compilation pipeline
cargo bench --bench end_to_end
```

### Benchmark Reports

Benchmark results are stored in `target/criterion/`. Open `target/criterion/report/index.html` in a browser to view HTML reports with:

- Performance comparisons over time
- Statistical analysis
- Throughput measurements
- Regression detection

## CPU Profiling

### Using perf (Linux)

Profile a compilation:

```bash
# Record profile
perf record -g cargo run --release -- compile examples/صنف.ترقيم

# View results
perf report
```

### Using flamegraph

Generate a flamegraph:

```bash
# Install if not already installed
cargo install flamegraph

# Profile compilation
cargo flamegraph -- compile examples/صنف.ترقيم

# Open flamegraph.svg in browser
```

### Using Instruments (macOS)

```bash
# Build with symbols
cargo build --release

# Run with Instruments
instruments -t "Time Profiler" target/release/tarqeem compile examples/صنف.ترقيم
```

## Memory Profiling

### Using heaptrack (Linux)

```bash
# Install
sudo apt-get install heaptrack

# Profile
heaptrack cargo run --release -- compile examples/صنف.ترقيم

# Analyze
heaptrack_gui heaptrack.tarqeem.*.gz
```

### Using DHAT (via valgrind)

```bash
cargo build --release
valgrind --tool=dhat target/release/tarqeem compile examples/صنف.ترقيم
```

## Profiling Feature Flag

Enable the `profiling` feature for additional instrumentation:

```bash
cargo build --release --features profiling
```

This enables compile-time instrumentation points that can be used with tracing tools.

## Benchmark Metrics

### Lexer Benchmarks

| Metric | Description |
|--------|-------------|
| `lexer_simple` | Tokens/second for simple variable declarations |
| `lexer_complex` | Tokens/second for complex expressions |
| `lexer_classes` | Tokens/second for class definitions |
| `lexer_real_world` | Real-world example file throughput |

### Parser Benchmarks

| Metric | Description |
|--------|-------------|
| `parser_simple` | AST nodes/second for declarations |
| `parser_expressions` | AST nodes/second for expressions |
| `parser_control_flow` | Nested if/else parsing speed |
| `parser_functions` | Function definition parsing |
| `parser_classes` | Class definition parsing |

### Semantic Benchmarks

| Metric | Description |
|--------|-------------|
| `semantic_typed_vars` | Type checking for typed variables |
| `semantic_function_calls` | Function call type resolution |
| `semantic_class_hierarchy` | Class inheritance resolution |
| `semantic_type_inference` | Type inference speed |

### IR Generation Benchmarks

| Metric | Description |
|--------|-------------|
| `ir_arithmetic` | IR generation for arithmetic |
| `ir_functions` | Function IR generation |
| `ir_control_flow` | Control flow IR generation |
| `ir_loops` | Loop IR generation |
| `ir_classes` | Class IR generation |

### Optimizer Benchmarks

| Metric | Description |
|--------|-------------|
| `optimizer_O0` | No optimization baseline |
| `optimizer_O1` | Basic optimizations |
| `optimizer_O2` | Standard optimizations |
| `optimizer_O3` | Aggressive optimizations |
| `optimizer_scalability` | Scaling with code size |

### End-to-End Benchmarks

| Metric | Description |
|--------|-------------|
| `end_to_end_hello` | Hello world compilation |
| `end_to_end_phases` | Phase breakdown |
| `end_to_end_scale` | Compilation scaling |
| `end_to_end_type` | By program type |
| `end_to_end_opt_levels` | Optimization level comparison |

## Performance Targets

| File Size | Target Time |
|-----------|-------------|
| 100 lines | <50ms |
| 1,000 lines | <200ms |
| 10,000 lines | <1s |

## Identifying Hotspots

Common hotspots to look for:

1. **String Operations**
   - Excessive cloning of identifiers
   - String concatenation in error messages
   - Unicode normalization overhead

2. **Hash Map Operations**
   - Symbol table lookups
   - Type cache misses
   - Scope chain traversal

3. **Memory Allocation**
   - AST node allocation
   - IR instruction creation
   - String interning (if not implemented)

4. **Type Checking**
   - Generic instantiation
   - Method resolution
   - Interface implementation checking

## Optimization Strategies

### Reduce Allocations

```rust
// Before: Clone on every lookup
let name = identifier.clone();

// After: Use references or interned strings
let name = &identifier;
// or
let name = self.interner.get(identifier);
```

### Cache Expensive Operations

```rust
// Before: Recompute every time
fn check_type(&self, ty: &Type) -> bool {
    self.compute_expensive_check(ty)
}

// After: Cache results
fn check_type(&mut self, ty: &Type) -> bool {
    if let Some(result) = self.type_cache.get(ty) {
        return *result;
    }
    let result = self.compute_expensive_check(ty);
    self.type_cache.insert(ty.clone(), result);
    result
}
```

### Use Arena Allocation

For AST nodes that are allocated together and freed together:

```rust
use bumpalo::Bump;

let arena = Bump::new();
let node = arena.alloc(AstNode::new(...));
```

## Continuous Performance Tracking

### CI Integration

Add benchmark regression testing to CI:

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench -- --save-baseline ci

- name: Compare benchmarks
  run: cargo bench -- --baseline ci
```

### Tracking Over Time

Store benchmark results and track trends:

```bash
# Save baseline
cargo bench -- --save-baseline v1.2.0

# Compare against baseline
cargo bench -- --baseline v1.2.0
```

## Troubleshooting

### Benchmark Variance

If benchmarks show high variance:

1. Close other applications
2. Disable CPU frequency scaling:
   ```bash
   sudo cpupower frequency-set --governor performance
   ```
3. Run with more iterations:
   ```bash
   cargo bench -- --sample-size 100
   ```

### Missing Symbols in Flamegraph

Build with debug info:

```bash
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph ...
```
