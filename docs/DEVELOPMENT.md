# rsdu Development Guide

## Getting Started

### Prerequisites

- Rust 1.70+ (2021 edition)
- Git
- A Unix-like terminal (Linux, macOS, WSL on Windows)

### Development Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd rsdu
   ```

2. **Install dependencies**
   ```bash
   cargo build
   ```

3. **Run tests**
   ```bash
   cargo test
   ```

4. **Run the application**
   ```bash
   cargo run -- --help
   ```

### Project Structure

```
rsdu/
├── src/
│   ├── main.rs          # Application entry point
│   ├── cli.rs           # Command-line interface
│   ├── config.rs        # Configuration management
│   ├── model.rs         # Data structures
│   ├── scanner.rs       # Directory scanning (stub)
│   ├── browser.rs       # Interactive UI (stub)
│   ├── ui.rs            # Terminal interface
│   ├── import.rs        # Data import
│   ├── export.rs        # Data export
│   ├── utils.rs         # Utility functions
│   └── error.rs         # Error handling
├── docs/
│   ├── README.md        # User documentation
│   ├── API.md           # API documentation
│   ├── IMPLEMENTATION.md # Implementation status
│   └── DEVELOPMENT.md   # This file
├── Cargo.toml           # Dependencies and metadata
└── .gitignore
```

## Development Workflow

### Building

```bash
# Debug build (fast compilation, slower execution)
cargo build

# Release build (slower compilation, fast execution)
cargo build --release

# Check without building (fastest)
cargo check
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests quietly
cargo test --quiet
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Run linter with all features
cargo clippy --all-features

# Check documentation
cargo doc --no-deps --open
```

## Implementation Priorities

### Phase 1: Core Scanning Engine (HIGH PRIORITY)

The scanner module (`src/scanner.rs`) needs complete implementation. This is the most critical missing piece.

**Required functionality:**
- Recursive directory traversal using `walkdir`
- Metadata collection (size, blocks, mtime, permissions)
- Error handling for inaccessible files
- Multi-threaded scanning with `rayon`
- Progress reporting with atomic counters
- Filesystem boundary detection
- Symlink handling
- Pattern-based exclusion

**Implementation approach:**
```rust
pub fn scan_directory(path: &Path, config: &Config) -> Result<Arc<Entry>> {
    // 1. Create root entry
    // 2. Set up thread pool with rayon
    // 3. Use walkdir for traversal
    // 4. Collect metadata for each entry
    // 5. Build tree structure
    // 6. Handle hardlinks
    // 7. Return completed tree
}
```

**Estimated effort:** 200-300 lines of code

### Phase 2: Hardlink Detection

Implement proper hardlink tracking to avoid double-counting disk usage.

**Key components:**
- `HardlinkMap` population during scan
- Deduplication logic in `Entry::shared_*` methods
- Cross-reference tracking for accurate statistics

**Files to modify:**
- `src/scanner.rs` - Detect and record hardlinks during scan
- `src/model.rs` - Complete hardlink tracking implementation

**Estimated effort:** 150-200 lines of code

### Phase 3: Interactive Browser (MEDIUM PRIORITY)

Complete the terminal-based browser interface in `src/browser.rs`.

**Required functionality:**
- Directory tree navigation (up/down, enter/back)
- Multiple display modes (size, blocks, items, mtime)
- Sorting options (name, size, date)
- Keyboard shortcuts (ncdu-compatible)
- Progress bars and visual indicators
- Color support
- Help screen

**Implementation approach:**
```rust
pub fn run_browser(root: Arc<Entry>, config: Config) -> Result<()> {
    // 1. Initialize terminal UI
    // 2. Set up main event loop
    // 3. Handle keyboard input
    // 4. Render current view
    // 5. Navigate directory tree
    // 6. Update display based on sorting
}
```

**Estimated effort:** 400-500 lines of code

### Phase 4: Binary Export Format

Implement ncdu-compatible binary export format.

**Requirements:**
- Exact compatibility with ncdu 2.x binary format
- Compression support with zstd
- Streaming export for large datasets
- Version detection and handling

**Files to modify:**
- `src/export.rs` - Complete binary export implementation
- `src/import.rs` - Complete binary import implementation

**Estimated effort:** 200-300 lines of code

## Code Style Guidelines

### Rust Conventions

1. **Follow standard Rust formatting**
   - Use `cargo fmt` to ensure consistency
   - 4-space indentation
   - 100-character line limit where practical

2. **Error handling**
   - Use `Result<T>` for fallible operations
   - Provide meaningful error messages with context
   - Use `?` operator for error propagation
   - Create specific error types for different failure modes

3. **Documentation**
   - Document all public APIs with `///` comments
   - Include examples in documentation where helpful
   - Use `//!` for module-level documentation

4. **Testing**
   - Write unit tests for all non-trivial functions
   - Use descriptive test names (`test_handles_permission_denied`)
   - Test both success and failure cases
   - Mock external dependencies where possible

### Project-Specific Guidelines

1. **Memory management**
   - Use `Arc<Entry>` for shared directory tree data
   - Prefer borrowing over cloning where possible
   - Use atomic types for statistics that cross thread boundaries

2. **Error handling pattern**
   ```rust
   use crate::error::{Result, RsduError};
   
   pub fn example_function() -> Result<()> {
       // Implementation
       Ok(())
   }
   ```

3. **Configuration access**
   - Pass `&Config` to functions that need configuration
   - Don't store global configuration state
   - Validate configuration early in the application

4. **Terminal UI**
   - Always clean up terminal state on exit
   - Handle SIGINT and other signals gracefully
   - Use crossterm for cross-platform compatibility

## Testing Strategy

### Unit Tests

Each module should have comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_specific_functionality() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### Integration Tests

Create end-to-end tests in `tests/` directory:

```rust
// tests/integration_test.rs
use rsdu::{Config, scanner};
use tempfile::tempdir;

#[test]
fn test_scan_directory_integration() {
    let temp_dir = tempdir().unwrap();
    // Create test directory structure
    // Run scanner
    // Verify results
}
```

### Performance Tests

Use criterion for benchmarking critical paths:

```rust
// benches/scan_benchmark.rs (when ready)
use criterion::{criterion_group, criterion_main, Criterion};

fn scan_benchmark(c: &mut Criterion) {
    c.bench_function("scan large directory", |b| {
        b.iter(|| {
            // Benchmark scanning operation
        })
    });
}

criterion_group!(benches, scan_benchmark);
criterion_main!(benches);
```

## Debugging

### Common Issues

1. **Terminal state corruption**
   - Always call UI cleanup in destructors
   - Test error paths that might skip cleanup
   - Use `RUST_BACKTRACE=1` to debug panics

2. **Memory usage**
   - Use `valgrind` or similar tools to check for leaks
   - Monitor memory usage with large directory trees
   - Profile with `cargo-profiler` if needed

3. **Cross-platform issues**
   - Test on Linux, macOS, and Windows
   - Be careful with path separators and Unicode
   - Use crossterm abstractions consistently

### Debug Builds

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Run with debug symbols and optimizations disabled
cargo build --profile=dev

# Use GDB or LLDB for debugging
rust-gdb target/debug/rsdu
```

## Performance Considerations

### Scalability Targets

- Handle directories with 1M+ entries
- Scan speed: >10,000 files/second on modern hardware
- Memory usage: <100MB for typical home directory (~100k files)
- Response time: <100ms for UI interactions

### Optimization Guidelines

1. **Scanning performance**
   - Use parallel iteration with rayon
   - Batch filesystem operations
   - Avoid unnecessary string allocations
   - Cache frequently accessed data

2. **Memory efficiency**
   - Use reference counting instead of cloning
   - Store strings as `OsString` to avoid UTF-8 validation
   - Pack structs to reduce memory overhead
   - Use atomic counters instead of mutexes where possible

3. **UI responsiveness**
   - Process input events in separate thread
   - Limit refresh rate to avoid overwhelming terminal
   - Use incremental updates for large directory listings

## Contributing

### Pull Request Process

1. **Create feature branch**
   ```bash
   git checkout -b feature/scanner-implementation
   ```

2. **Implement changes**
   - Follow coding guidelines
   - Add comprehensive tests
   - Update documentation

3. **Quality checks**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

4. **Commit messages**
   Use conventional commit format:
   ```
   feat(scanner): implement parallel directory traversal
   fix(ui): prevent terminal corruption on panic
   docs(api): add examples for Entry methods
   ```

### Code Review Checklist

- [ ] Code follows project style guidelines
- [ ] All tests pass
- [ ] New functionality has tests
- [ ] Documentation is updated
- [ ] No compiler warnings
- [ ] Performance implications considered
- [ ] Error handling is appropriate
- [ ] Memory safety verified

## Release Process

### Version Management

Follow semantic versioning (semver):
- `0.1.0` - Initial implementation with core scanning
- `0.2.0` - Interactive browser interface
- `0.3.0` - Binary format support
- `1.0.0` - Feature-complete ncdu replacement

### Release Checklist

1. [ ] All tests pass on supported platforms
2. [ ] Performance benchmarks meet targets
3. [ ] Documentation is up to date
4. [ ] Examples work correctly
5. [ ] Version number updated in Cargo.toml
6. [ ] Changelog updated
7. [ ] Git tag created
8. [ ] Release published

## Troubleshooting

### Common Build Issues

1. **Missing system dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install build-essential libssl-dev pkg-config
   
   # macOS
   xcode-select --install
   ```

2. **Rust version issues**
   ```bash
   rustup update
   rustup show  # Verify version
   ```

3. **Cross-compilation**
   ```bash
   # Add target
   rustup target add x86_64-unknown-linux-musl
   
   # Build for target
   cargo build --target x86_64-unknown-linux-musl --release
   ```

### Performance Issues

1. **Slow compilation**
   - Use `cargo check` instead of `cargo build` during development
   - Enable incremental compilation (enabled by default in newer Rust)
   - Consider using `sccache` for caching builds

2. **Slow execution**
   - Profile with `cargo-profiler` or `perf`
   - Check for unnecessary allocations
   - Verify parallel execution is working

### Testing Issues

1. **Flaky tests**
   - Avoid depending on system state
   - Use temp directories for file system tests
   - Mock external dependencies

2. **Platform-specific failures**
   - Test on all supported platforms
   - Use conditional compilation for platform-specific code
   - Handle platform differences gracefully

## Resources

### Documentation

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Documentation](https://doc.rust-lang.org/std/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [ncdu source code](https://code.blicky.net/yorhel/ncdu) - Reference implementation

### Dependencies

- [clap documentation](https://docs.rs/clap/) - CLI parsing
- [crossterm documentation](https://docs.rs/crossterm/) - Terminal control
- [rayon documentation](https://docs.rs/rayon/) - Parallel processing
- [walkdir documentation](https://docs.rs/walkdir/) - Directory traversal
- [serde documentation](https://docs.rs/serde/) - Serialization

### Tools

- [rust-analyzer](https://rust-analyzer.github.io/) - IDE support
- [cargo-watch](https://github.com/watchexec/cargo-watch) - Auto-rebuild
- [cargo-audit](https://github.com/RustSec/rustsec) - Security audits
- [cargo-outdated](https://github.com/kbknapp/cargo-outdated) - Dependency updates

## Contact

For questions about development:
- Check existing issues and documentation
- Create detailed issues for bugs or feature requests
- Include minimal reproduction cases
- Provide environment information (OS, Rust version, etc.)