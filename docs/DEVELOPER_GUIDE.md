# Aethyro NTG: Developer Guide

**Version:** 1.0  
**Date:** 2026-07-16  
**Audience:** Engineers, Researchers, Contributors  

---

## Quick Start for Developers

### Environment Setup

```bash
# Clone repo
git clone https://github.com/yourorg/aethyro-ntg.git
cd aethyro-ntg

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Add SIMD target support
rustup component add rustfmt clippy
cargo install cargo-critertion

# Verify
cargo --version   # cargo 1.75+
rustc --version   # rustc 1.75+
```

### Build & Test

```bash
# Navigate to kernel
cd kernel

# Build (debug)
cargo build

# Build (release, optimized)
cargo build --release

# Run tests
cargo test

# Run tests with logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_ternary_matmul

# Benchmark
cargo bench
```

### IDE Setup

**VS Code + rust-analyzer:**

```json
// .vscode/settings.json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": ["--all-targets"],
  "rust-analyzer.inlayHints.enable": true,
  "rust-analyzer.inlayHints.typeHints.enable": true
}
```

**JetBrains IntelliJ:**

- Install Rust plugin
- Settings → Languages & Frameworks → Rust → Cargo: select `/path/to/kernel/Cargo.toml`

---

## 1. Component Architecture

### 1.1 Crate Structure

```
kernel/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── main.rs             # HTTP server entrypoint
│   ├── ternary.rs          # Ternary quantization & matmul
│   ├── storage.rs          # Weight serialization
│   ├── graph.rs            # Graph topology
│   ├── ledger.rs           # Audit ledger
│   ├── runtime.rs          # Request dispatch & scheduling
│   ├── api.rs              # HTTP endpoints
│   ├── activations.rs      # Activation functions
│   ├── cpu_features.rs     # SIMD detection
│   └── bin/                # Binaries (tools, demos)
├── tests/                  # Integration tests
├── benches/                # Benchmarks
├── Cargo.toml
└── target/                 # Build artifacts (git-ignored)
```

### 1.2 Module Dependencies

```
api.rs
  └─> runtime.rs
       ├─> graph.rs
       │    └─> ternary.rs, storage.rs
       ├─> ledger.rs
       ├─> activations.rs
       └─> cpu_features.rs

main.rs
  └─> api.rs
```

---

## 2. Development Workflows

### 2.1 Adding a New Feature

**Example: Custom activation function**

1. **Create trait extension** (in `src/activations.rs`):

```rust
pub trait Activation: Send + Sync {
    fn forward(&self, x: f32) -> f32;
    fn backward(&self, grad: f32, x: f32) -> f32;
}

pub struct ReLU;
impl Activation for ReLU {
    fn forward(&self, x: f32) -> f32 {
        x.max(0.0)
    }
    
    fn backward(&self, grad: f32, x: f32) -> f32 {
        if x > 0.0 { grad } else { 0.0 }
    }
}
```

2. **Add tests**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relu_forward() {
        let relu = ReLU;
        assert_eq!(relu.forward(0.5), 0.5);
        assert_eq!(relu.forward(-0.5), 0.0);
    }

    #[test]
    fn test_relu_backward() {
        let relu = ReLU;
        assert_eq!(relu.backward(1.0, 0.5), 1.0);
        assert_eq!(relu.backward(1.0, -0.5), 0.0);
    }
}
```

3. **Integrate into graph**:

```rust
// In src/graph.rs
impl Graph {
    pub fn apply_activation(&mut self, node_id: usize, act: Box<dyn Activation>) -> Result<()> {
        self.nodes[node_id].activation = Some(act);
        Ok(())
    }
}
```

4. **Document in ADR**:

```markdown
# ADR 0011: Custom Activation Functions

**Status:** Accepted

## Problem
Users need extensible activation functions beyond sigmoid/tanh.

## Decision
Implement trait-based activation system with runtime dispatch.

## Consequences
- Minimal performance overhead (vtable lookup)
- Users can implement custom activations
```

5. **Run full test suite**:

```bash
cargo test --all
cargo clippy --all
cargo fmt --check
```

### 2.2 Performance Optimization

**Workflow:**

1. **Benchmark baseline**:

```bash
cargo bench --bench simd_benchmark
# baseline: 201000 ops/sec
```

2. **Profile with perf** (Linux):

```bash
cargo build --release
perf record -F 99 -g ./target/release/density_bench
perf report
```

3. **Implement optimization**:

```rust
// Example: SIMD optimization
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub fn matmul_avx2(a: &[f32], w: &[i8], b: &[f32]) -> Vec<f32> {
    // SIMD implementation
}
```

4. **Benchmark after**:

```bash
cargo bench --bench simd_benchmark
# optimized: 2400000 ops/sec (12x improvement)
```

5. **Document findings**:

```markdown
# Performance Improvement: AVX2 TOBL

- Baseline: 201K ops/sec
- Optimized: 2.4M ops/sec
- Improvement: 12x
- Tested on: Intel i7-9700K (8 cores)
```

### 2.3 Testing Strategy

**Unit Tests** (in same file):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_quantization() {
        let weights = vec![0.5, -0.8, 0.1];
        let ternary = quantize_to_ternary(&weights);
        assert_eq!(ternary, vec![1, -1, 0]);
    }
}
```

**Integration Tests** (in `tests/` directory):

```rust
// tests/phase1_2_3_integration.rs
use ntg_kernel::*;

#[test]
fn test_end_to_end_inference() {
    let graph = Graph::load("models/test.ntg").unwrap();
    let input = vec![0.5; 1024];
    let output = graph.forward(&input).unwrap();
    assert_eq!(output.len(), 3);
}
```

**Benchmarks** (in `benches/` directory):

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn matmul_benchmark(c: &mut Criterion) {
    c.bench_function("matmul_scalar_1024", |b| {
        b.iter(|| matmul_scalar(
            black_box(&input),
            black_box(&weights),
            black_box(&bias)
        ))
    });
}

criterion_group!(benches, matmul_benchmark);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench
cargo bench -- --verbose
cargo bench -- --sample-size 1000
```

---

## 3. Testing Guide

### 3.1 Running Test Suites

```bash
# Run all tests
cargo test --all

# Run specific test file
cargo test --test phase1_2_3_integration

# Run specific test
cargo test test_ternary_matmul

# Run tests and show output
cargo test -- --nocapture

# Run with specific log level
RUST_LOG=debug cargo test -- --nocapture

# Run benchmark tests
cargo test --release --benches
```

### 3.2 Test Coverage

```bash
# Install tarpaulin (code coverage tool)
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin -o Html --out coverage

# View report
open coverage/index.html
```

### 3.3 Continuous Integration

Add to `.github/workflows/ci.yml`:

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: 1.75
    
    - name: Run tests
      run: cargo test --all
    
    - name: Run clippy
      run: cargo clippy --all -- -D warnings
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Run benchmarks
      run: cargo bench --no-run
```

---

## 4. Debugging

### 4.1 Debug Logging

```rust
// Enable logging in your code
use log::{debug, info, warn, error};

fn process_input(input: &[f32]) -> Result<Vec<f32>> {
    debug!("Processing input of size {}", input.len());
    
    if input.is_empty() {
        warn!("Empty input received");
        return Err("Input cannot be empty".into());
    }
    
    info!("Successfully processed input");
    Ok(vec![])
}
```

Run with debug output:

```bash
RUST_LOG=debug cargo run --release
RUST_LOG=ntg_kernel=debug,hyper=info cargo run
```

### 4.2 Error Handling

```rust
// Use Result types for error propagation
fn infer(graph: &Graph, input: &[f32]) -> Result<Vec<f32>, InferenceError> {
    if graph.is_empty() {
        return Err(InferenceError::GraphNotLoaded);
    }
    
    if input.len() != graph.input_size() {
        return Err(InferenceError::ShapeMismatch {
            expected: graph.input_size(),
            received: input.len(),
        });
    }
    
    graph.forward(input)
}

// Define custom error type
#[derive(Debug)]
pub enum InferenceError {
    GraphNotLoaded,
    ShapeMismatch { expected: usize, received: usize },
    ComputeError(String),
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::GraphNotLoaded => write!(f, "Graph not loaded"),
            Self::ShapeMismatch { expected, received } => {
                write!(f, "Shape mismatch: expected {}, got {}", expected, received)
            }
            Self::ComputeError(e) => write!(f, "Compute error: {}", e),
        }
    }
}
```

### 4.3 Inspecting Runtime State

```rust
// Add debug methods to structs
impl Graph {
    pub fn debug_info(&self) -> String {
        format!(
            "Graph {{ nodes: {}, edges: {}, density: {:.2}% }}",
            self.nodes.len(),
            self.edge_count(),
            self.density() * 100.0
        )
    }
}

// Use in logging
info!("{}", graph.debug_info());
```

---

## 5. Profiling & Optimization

### 5.1 CPU Profiling (Linux perf)

```bash
# Build with debug symbols
cargo build --release

# Record profile
perf record -F 99 -g --call-graph=dwarf ./target/release/kernel_host

# View results
perf report

# Export flame graph
perf script > perf.out
# Use: https://www.brendangregg.com/flamegraph.html
```

### 5.2 Memory Profiling (Valgrind)

```bash
valgrind --leak-check=full \
  --show-leak-kinds=all \
  --track-origins=yes \
  ./target/release/kernel_host

# Or with heaptrack
heaptrack ./target/release/kernel_host
heaptrack_gui heaptrack.kernel_host.*.gz
```

### 5.3 Benchmarking Different Configurations

```rust
// In benches/compare.rs
use criterion::{Criterion, BenchmarkId};

fn compare_matmul(c: &mut Criterion) {
    let mut group = c.benchmark_group("matmul");
    
    for size in [128, 256, 512, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let input = vec![0.5; size];
                let weights = vec![1; size * size];
                b.iter(|| matmul_scalar(&input, &weights, &[0.0; size]))
            }
        );
    }
}
```

Run:

```bash
cargo bench -- --verbose
```

---

## 6. Extending the System

### 6.1 Adding a New Data Format

```rust
// In src/storage.rs
pub trait WeightCodec: Send + Sync {
    fn encode(&self, weights: &[f32]) -> Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> Result<Vec<f32>>;
    fn compression_ratio(&self) -> f32;
}

// Implement for new format
pub struct Fp8Codec;

impl WeightCodec for Fp8Codec {
    fn encode(&self, weights: &[f32]) -> Result<Vec<u8>> {
        weights.iter()
            .map(|&w| quantize_fp8(w))
            .collect()
    }
    
    fn decode(&self, bytes: &[u8]) -> Result<Vec<f32>> {
        bytes.iter()
            .map(|&b| dequantize_fp8(b))
            .collect()
    }
    
    fn compression_ratio(&self) -> f32 {
        0.25  // 4 bytes -> 1 byte
    }
}
```

### 6.2 Adding Custom Ledger Backend

```rust
// In src/ledger.rs
pub trait LedgerBackend: Send + Sync {
    fn append(&mut self, block: LedgerBlock) -> Result<()>;
    fn verify(&self) -> Result<()>;
    fn get_height(&self) -> u64;
}

// Implement Redis backend
pub struct RedisLedger {
    client: redis::Client,
}

impl LedgerBackend for RedisLedger {
    fn append(&mut self, block: LedgerBlock) -> Result<()> {
        let json = serde_json::to_string(&block)?;
        self.client.rpush("ledger", json)?;
        Ok(())
    }
    
    fn verify(&self) -> Result<()> {
        // Verify chain integrity via Redis
        Ok(())
    }
    
    fn get_height(&self) -> u64 {
        self.client.llen("ledger").unwrap_or(0)
    }
}
```

---

## 7. Documentation Standards

### 7.1 Code Comments

```rust
/// Computes ternary quantization of floating-point weights.
///
/// # Arguments
/// * `weights` - Vector of FP32 weights to quantize
/// * `seed` - Random seed for tie-breaking (deterministic)
///
/// # Returns
/// Vector of ternary values {-1, 0, +1}
///
/// # Examples
/// ```
/// let weights = vec![0.5, -0.8, 0.1];
/// let ternary = quantize_to_ternary(&weights, 42);
/// assert_eq!(ternary, vec![1, -1, 0]);
/// ```
///
/// # Performance
/// Time complexity: O(n)
/// Space complexity: O(n)
pub fn quantize_to_ternary(weights: &[f32], seed: u64) -> Vec<i8> {
    // Implementation
}
```

### 7.2 Module Documentation

```rust
//! Ternary Core Module
//!
//! Provides ternary quantization and matrix multiplication kernels.
//! Supports both scalar and SIMD implementations with automatic
//! acceleration selection.
//!
//! # Example
//! ```
//! let weights = quantize_to_ternary(&fp32_weights, seed);
//! let result = matmul_ternary(&input, &weights, &bias)?;
//! ```

pub mod ternary;
pub mod simd;
pub mod cpu_features;
```

### 7.3 Architecture Decision Records

Create `docs/architecture/000N-description.md`:

```markdown
# ADR 000N: Feature Name

**Status:** Proposed / Accepted / Deprecated

**Date:** 2026-07-16

## Problem
Clear statement of the problem to be solved.

## Decision
Concise statement of the decision made.

## Rationale
Why this decision was chosen over alternatives.

## Consequences
Positive and negative outcomes of this decision.

## Alternatives Considered
- Alternative 1: Why rejected?
- Alternative 2: Why rejected?

## References
- [Related ADR](#)
- [External resource](#)
```

---

## 8. Contributing Guidelines

### 8.1 Before You Submit

- [ ] Code passes `cargo clippy --all` with no warnings
- [ ] Code formatted with `cargo fmt`
- [ ] Tests pass: `cargo test --all`
- [ ] Documentation updated
- [ ] Commit message follows conventions

### 8.2 Commit Message Template

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

**Example:**

```
feat(ternary): Add FP8 quantization codec

Implements new FP8 weight codec with 4x compression.
- Add Fp8Codec struct implementing WeightCodec trait
- Add tests for encode/decode correctness
- Benchmark shows 2x throughput improvement vs FP32

Closes #42
```

### 8.3 Pull Request Checklist

- [ ] Branch created from `main`
- [ ] Commit messages follow template
- [ ] Code reviewed by 2+ maintainers
- [ ] CI checks pass
- [ ] Changelog updated
- [ ] Ready for merge

---

## 9. Release Process

### 9.1 Version Bumping

```bash
# Update Cargo.toml
cargo bump minor  # 0.1.0 -> 0.2.0

# Run full test suite
cargo test --all
cargo clippy --all
cargo bench

# Create release commit
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.2.0"

# Create tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push
git push origin main --tags
```

### 9.2 Build Release Artifacts

```bash
# Build optimized binaries
cargo build --release

# Create distribution archives
tar -czf aethyro-ntg-linux-x86_64.tar.gz \
  -C target/release kernel_host

# Generate checksum
sha256sum aethyro-ntg-linux-x86_64.tar.gz > checksum.sha256

# Upload to release page
gh release create v0.2.0 \
  aethyro-ntg-linux-x86_64.tar.gz \
  checksum.sha256
```

---

## 10. Resources

- **Book:** [The Rust Programming Language](https://doc.rust-lang.org/book/)
- **API Docs:** `cargo doc --open`
- **Benchmarking:** [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- **Profiling:** [Flamegraph](https://www.brendangregg.com/flamegraph.html)
- **SIMD:** [packed_simd](https://rust-lang.github.io/packed_simd_2/)

---

## Version History

- **v1.0 (2026-07-16):** Production developer guide
- **v0.9 (2026-07-09):** Development workflow draft

