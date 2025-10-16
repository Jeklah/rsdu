# rsdu - Rust Disk Usage Analyzer

A fast, interactive disk usage analyzer written in Rust, inspired by `ncdu` (NCurses Disk Usage). This tool provides both command-line scanning capabilities and a terminal-based interactive browser for exploring directory structures and disk usage.

## Features

### ✅ Implemented
- **Fast Directory Scanning**: Multi-threaded directory traversal with comprehensive metadata collection
- **Interactive TUI Browser**: Terminal-based interface with keyboard navigation
- **File System Support**: 
  - Hardlink detection and tracking
  - Symbolic link handling
  - Cross-filesystem boundary detection
  - Linux pseudo-filesystem exclusion (proc, sys, dev, etc.)
- **Flexible Configuration**: Command-line arguments with extensive customization options
- **Smart Filtering**:
  - Pattern-based file exclusion (glob patterns)
  - Cache directory detection (CACHEDIR.TAG)
  - Hidden file handling
- **Extended Metadata**: Optional collection of timestamps, permissions, ownership
- **Multiple Display Modes**: Size formatting (SI/binary), sorting options
- **Error Handling**: Graceful handling of permission errors and inaccessible files

### 🚧 Partially Implemented
- **Interactive Browser**: Basic navigation works, but has display refresh issues
- **Export/Import**: Framework exists but needs completion
- **Configuration Files**: Parsing logic implemented but not fully integrated

### ❌ Not Yet Implemented
- Background/color schemes
- File deletion capabilities
- Directory refresh
- Shell integration
- Progress indicators during scan
- JSON/binary export formats

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd rsdu

# Build the project
cargo build --release

# Run
./target/release/rsdu [directory]
```

## Usage

### Basic Scanning
```bash
# Scan current directory
rsdu

# Scan specific directory
rsdu /path/to/directory

# Scan with extended metadata
rsdu -e /path/to/directory

# Multi-threaded scanning
rsdu -t 8 /path/to/directory
```

### Interactive Browser Keys
- `↑/k` - Move up
- `↓/j` - Move down  
- `←/h` - Go back to parent directory
- `→/l/Enter` - Enter selected directory
- `PgUp/PgDn` - Page up/down
- `Home/g` - Go to first item
- `End/G` - Go to last item
- `?/F1` - Toggle help
- `q/Esc` - Quit
- `Ctrl+C` - Force quit

### Command Line Options

#### Scanning Options
- `-x, --one-file-system` - Stay on same filesystem
- `-e, --extended` - Show extended information (mtime, permissions, etc.)
- `-L, --follow-symlinks` - Follow symbolic links
- `--exclude PATTERN` - Exclude files matching pattern
- `-X, --exclude-from FILE` - Exclude patterns from file
- `--exclude-caches` - Exclude directories with CACHEDIR.TAG
- `--exclude-kernfs` - Exclude Linux pseudo filesystems
- `-t, --threads NUM` - Number of threads for scanning

#### Display Options  
- `--si` - Use SI (base 10) prefixes instead of binary
- `--apparent-size` - Show apparent size instead of disk usage
- `--show-hidden` - Show hidden files by default
- `--sort COLUMN` - Sort by column (name, disk-usage, apparent-size, itemcount, mtime)

#### Export/Import Options
- `-o, --output FILE` - Export to JSON file
- `-O, --output-binary FILE` - Export to binary file  
- `-f, --file FILE` - Import previously scanned data
- `-c, --compress` - Use compression for export

#### UI Options
- `-0, --no-ui` - No UI during scan
- `-1, --line-ui` - Minimal line UI during scan
- `-2, --full-ui` - Full ncurses UI during scan

## Architecture

The project is organized into several modules:

- **`main.rs`** - Entry point and application flow coordination
- **`cli.rs`** - Command-line argument parsing and validation
- **`config.rs`** - Configuration management and file loading
- **`scanner.rs`** - Directory scanning and file system traversal
- **`browser.rs`** - Interactive terminal user interface
- **`model.rs`** - Data structures for file system representation
- **`error.rs`** - Error types and handling
- **`utils.rs`** - Utility functions (formatting, path handling, etc.)
- **`export.rs`** - Data export functionality (JSON/binary)
- **`import.rs`** - Data import functionality
- **`ui.rs`** - Lower-level terminal interface utilities

## Performance

rsdu is designed for performance:

- **Multi-threaded scanning** - Configurable thread pool for parallel directory traversal
- **Efficient data structures** - Optimized tree representation with Arc<> for shared ownership
- **Memory-conscious** - Streaming processing where possible
- **Fast sorting** - Natural string sorting and configurable sort criteria

## Example Output

```
Scanning directory: /home/user/project

Scan complete:
  Directories: 1,234
  Files: 5,678
  Total entries: 6,912
  Errors: 2
  Total size: 1,234,567,890 bytes
  Total blocks: 2,468,135

=== Interactive Browser ===
    Size    Items  Name
/project
  1.2 GB      11 /node_modules
  45.3 MB     156 /src  
  12.1 MB      23 /dist
  8.7 MB       45 /docs
  2.3 MB        8 /assets
  1.1 MB        3 package.json
  567 KB       12 README.md
  123 KB        1 .gitignore

1/8 items, 8 total | q:quit ?:help ↑↓:navigate ←→:enter/back
```

## Dependencies

- **clap** - Command-line argument parsing
- **crossterm** - Cross-platform terminal manipulation  
- **serde** - Serialization framework
- **rayon** - Data parallelism
- **humansize** - Human-readable size formatting
- **chrono** - Date and time handling
- **walkdir** - Directory traversal
- **glob** - Pattern matching
- **indicatif** - Progress indicators

## Contributing

Contributions are welcome! Areas that need work:

1. **Browser UI improvements** - Fix display refresh issues, add color schemes
2. **Export/Import completion** - Finish JSON and binary format support  
3. **Configuration files** - Complete config file loading and merging
4. **Performance optimization** - Profile and optimize hot paths
5. **Testing** - Expand test coverage, especially for edge cases
6. **Documentation** - API documentation and usage examples

## License

This project is licensed under the MIT License. See LICENSE file for details.

## Acknowledgments

Inspired by the excellent `ncdu` tool by Yoran Heling. This project aims to provide similar functionality with the performance and safety benefits of Rust.