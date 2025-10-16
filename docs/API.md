# rsdu API Documentation

## Overview

This document provides comprehensive API documentation for rsdu's internal modules and public interfaces. While rsdu is primarily a command-line application, its modular design makes it suitable for use as a library for disk usage analysis.

## Core Modules

### `main.rs` - Application Entry Point

#### Functions

```rust
fn main() -> Result<()>
```
Main application entry point that handles CLI parsing, configuration loading, and application flow coordination.

```rust
fn handle_import(import_file: &str, config: &Config) -> Result<()>
```
Handles importing previously scanned data from JSON or binary formats.

```rust
fn run_application(scan_path: PathBuf, config: Config) -> Result<()>
```
Main application flow for scanning and browsing directories.

### `cli.rs` - Command Line Interface

#### Structures

```rust
pub struct Args
```
Complete command-line argument structure using `clap::Parser` derive macro.

**Key Fields:**
- `directory: Option<PathBuf>` - Directory to scan
- `import_file: Option<String>` - Import from file
- `export_json: Option<String>` - Export to JSON
- `threads: Option<usize>` - Number of scanning threads
- `exclude: Vec<String>` - Exclusion patterns

#### Enums

```rust
pub enum GraphStyle
```
Graph visualization styles for usage bars.

**Variants:**
- `Hash` - ASCII hash marks (`#`)
- `HalfBlock` - Unicode half-block characters
- `EighthBlock` - Unicode eighth-block characters

```rust
pub enum SharedColumn
```
Display modes for shared/hardlinked content.

**Variants:**
- `Off` - Don't show shared column
- `Shared` - Show shared space
- `Unique` - Show unique space only

```rust
pub enum ColorScheme
```
Terminal color schemes.

**Variants:**
- `Off` - No colors
- `Dark` - Dark theme
- `DarkBg` - Dark background theme

#### Methods

```rust
impl Args {
    pub fn validate(&self) -> Result<(), String>
}
```
Validates command-line arguments for conflicts and invalid values.

### `config.rs` - Configuration Management

#### Structures

```rust
pub struct Config
```
Main configuration structure containing all application settings.

**Key Fields:**
- `same_fs: bool` - Stay on same filesystem
- `extended: bool` - Show extended information
- `threads: usize` - Number of scanning threads
- `exclude_patterns: Vec<String>` - File exclusion patterns
- `sort_col: SortColumn` - Default sort column
- `sort_order: SortOrder` - Sort direction

#### Enums

```rust
pub enum ScanUi
```
UI modes during directory scanning.

**Variants:**
- `None` - No UI output
- `Line` - Single-line progress
- `Full` - Full ncurses interface

```rust
pub enum SortColumn
```
Available sort columns.

**Variants:**
- `Name` - Sort by filename
- `Blocks` - Sort by disk usage
- `Size` - Sort by apparent size
- `Items` - Sort by item count
- `Mtime` - Sort by modification time

```rust
pub enum SortOrder
```
Sort direction options.

**Variants:**
- `Asc` - Ascending order
- `Desc` - Descending order

#### Methods

```rust
impl Config {
    pub fn from_args(args: &Args) -> Result<Self>
    pub fn default() -> Self
}
```

### `model.rs` - Data Model

#### Core Types

```rust
pub type EntryId = u64
pub type DeviceId = u32
pub type InodeId = u64
pub const BLOCK_SIZE: u64 = 512
```

#### Structures

```rust
pub struct Entry
```
Core file system entry representation.

**Fields:**
- `id: EntryId` - Unique entry identifier
- `entry_type: EntryType` - Type of file system object
- `name: OsString` - File/directory name
- `size: u64` - Apparent size in bytes
- `blocks: u64` - Disk usage in 512-byte blocks
- `device: DeviceId` - Device identifier
- `inode: InodeId` - Inode number
- `nlink: u32` - Hard link count
- `extended: Option<ExtendedInfo>` - Optional extended metadata
- `error: Option<String>` - Error message if inaccessible
- `children: Vec<Arc<Entry>>` - Child entries for directories
- `parent: Option<std::sync::Weak<Entry>>` - Parent reference

**Methods:**
```rust
impl Entry {
    pub fn new(id: EntryId, entry_type: EntryType, name: OsString, 
               size: u64, blocks: u64, device: DeviceId, 
               inode: InodeId, nlink: u32) -> Self
    
    pub fn error(id: EntryId, name: OsString, error: String) -> Self
    pub fn full_path(&self) -> PathBuf
    pub fn name_str(&self) -> String
    pub fn has_error(&self) -> bool
    pub fn has_sub_error(&self) -> bool
    pub fn add_child(&mut self, child: Entry) -> Arc<Entry>
    pub fn total_size(&self) -> u64
    pub fn total_blocks(&self) -> u64
    pub fn total_items(&self) -> u64
    pub fn shared_size(&self, hardlink_map: &HardlinkMap) -> u64
    pub fn shared_blocks(&self, hardlink_map: &HardlinkMap) -> u64
    pub fn sort_children(&mut self, sort_col: SortColumn, 
                         sort_order: SortOrder, dirs_first: bool)
    pub fn to_serializable(&self) -> SerializableEntry
    pub fn from_serializable(serializable: SerializableEntry) -> Arc<Self>
}
```

```rust
pub struct SerializableEntry
```
Serialization-friendly version of Entry for JSON export/import.

**Fields:**
- All Entry fields except `children` use simple types
- `name: String` instead of `OsString`
- `children: Vec<SerializableEntry>` for recursive structure

```rust
pub struct ExtendedInfo
```
Extended file metadata when `-e` flag is used.

**Fields:**
- `mtime: Option<DateTime<Utc>>` - Modification time
- `uid: Option<u32>` - User ID
- `gid: Option<u32>` - Group ID  
- `mode: Option<u32>` - File permissions

```rust
pub struct ScanStats
```
Thread-safe statistics during scanning.

**Fields:**
- `total_entries: AtomicU64` - Total entries processed
- `directories: AtomicU64` - Directory count
- `files: AtomicU64` - File count
- `errors: AtomicU64` - Error count
- `total_size: AtomicU64` - Total size in bytes
- `total_blocks: AtomicU64` - Total blocks used

#### Enums

```rust
pub enum EntryType
```
File system entry types.

**Variants:**
- `Directory` - Regular directory
- `File` - Regular file
- `Symlink` - Symbolic link
- `Hardlink` - Hard link (multiple names)
- `Special` - Device, pipe, socket, etc.
- `Error` - Inaccessible entry
- `Excluded` - Excluded by pattern
- `OtherFs` - Different filesystem
- `KernelFs` - Kernel filesystem (proc, sys, etc.)

#### Hardlink Support

```rust
pub struct HardlinkKey
pub struct HardlinkInfo
pub type HardlinkMap = HashMap<HardlinkKey, HardlinkInfo>
```

Structures for tracking and deduplicating hard links.

#### Utility Functions

```rust
pub fn generate_entry_id() -> EntryId
```
Generates unique entry identifiers.

### `scanner.rs` - Directory Scanning

#### Functions

```rust
pub fn scan_directory(path: &Path, config: &Config) -> Result<Arc<Entry>>
```
Main directory scanning function. Currently a stub returning minimal data.

**Parameters:**
- `path` - Root directory to scan
- `config` - Scanning configuration

**Returns:**
- `Arc<Entry>` - Root entry with scanned tree

### `browser.rs` - Interactive Browser

#### Functions

```rust
pub fn run_browser(root: Arc<Entry>, config: Config) -> Result<()>
```
Interactive browser interface. Currently a stub showing basic information.

**Parameters:**
- `root` - Root entry to browse
- `config` - Display configuration

### `tui.rs` - Terminal User Interface

#### Structs

```rust
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
    mode: AppMode,
}

pub struct ScanProgress {
    pub current_path: Mutex<String>,
    pub total_entries: AtomicUsize,
    pub directories: AtomicUsize,
    pub files: AtomicUsize,
    pub errors: AtomicUsize,
    pub total_size: AtomicUsize,
    pub is_complete: AtomicBool,
}
```

#### Methods

```rust
impl TuiApp {
    pub fn new(config: Config) -> Result<Self>
    pub fn start_scan(&mut self, scan_path: String) -> Result<Sender<ScanMessage>>
    pub fn run(&mut self) -> Result<()>
}
```

#### Enums

```rust
pub enum AppMode {
    Scanning { progress: Arc<ScanProgress>, receiver: Option<Receiver<ScanMessage>> },
    Browsing { root: Arc<Entry>, current_dir: Arc<Entry>, path_stack: Vec<Arc<Entry>>, list_state: ListState, show_help: bool },
    Quit,
}

pub enum ScanMessage {
    Progress { current_path: String, stats: ProgressStats },
    Complete { root: Arc<Entry> },
    Error { message: String },
}
```

### `import.rs` - Data Import

#### Functions

```rust
pub fn import_from_stdin() -> Result<Arc<Entry>>
pub fn import_from_file(path: &Path) -> Result<Arc<Entry>>
pub fn import_from_json(json: &str) -> Result<Arc<Entry>>
pub fn import_from_binary(data: &[u8]) -> Result<Arc<Entry>>
```

Import functions for various data sources and formats.

### `export.rs` - Data Export

#### Structures

```rust
pub struct ExportHandler
```
Handles data export to various formats.

**Methods:**
```rust
impl ExportHandler {
    pub fn json<W: Write + Send + 'static>(writer: W, compress: bool) -> Self
    pub fn binary<W: Write + Send + 'static>(writer: W, compress: bool) -> Self
    pub fn export(&mut self, entry: &Entry) -> Result<()>
}
```

#### Enums

```rust
pub enum ExportFormat
```
**Variants:**
- `Json` - JSON format export
- `Binary` - Binary format export (ncdu-compatible)

#### Functions

```rust
pub fn setup_json_export(filename: &str) -> Result<ExportHandler>
pub fn setup_binary_export(filename: &str) -> Result<ExportHandler>
pub fn export_to_json_string(entry: &Entry) -> Result<String>
pub fn export_to_json_compact(entry: &Entry) -> Result<String>
```

### `utils.rs` - Utility Functions

#### Formatting Functions

```rust
pub fn format_file_size(size: u64, use_si: bool) -> String
pub fn format_blocks(blocks: u64, use_si: bool) -> String
pub fn format_percentage(part: u64, total: u64) -> String
pub fn format_number_with_separator(num: u64, separator: &str) -> String
```

#### File System Utilities

```rust
pub fn is_hidden_file<P: AsRef<Path>>(path: P) -> bool
pub fn expand_user_path<P: AsRef<Path>>(path: P) -> Result<PathBuf>
pub fn matches_glob_pattern(path: &str, pattern: &str) -> bool
```

#### Display Utilities

```rust
pub fn create_progress_bar(percentage: f64, width: usize, style: &str) -> String
pub fn truncate_string(s: &str, max_width: usize) -> String
pub fn pad_string(s: &str, width: usize, right_align: bool) -> String
pub fn escape_for_display(s: &str) -> String
```

#### Sorting Utilities

```rust
pub fn natural_compare(a: &str, b: &str) -> std::cmp::Ordering
```

#### System Information

```rust
pub fn get_terminal_size() -> (usize, usize)
pub fn get_cpu_count() -> usize
pub fn is_running_in_container() -> bool
```

### `error.rs` - Error Handling

#### Main Error Type

```rust
pub enum RsduError
```
Comprehensive error type for all rsdu operations.

**Variants:**
- `Io(io::Error)` - I/O errors
- `PermissionDenied { path: PathBuf, source: io::Error }` - Access denied
- `PathNotFound { path: PathBuf }` - Path doesn't exist
- `InvalidPath { path: PathBuf, reason: String }` - Invalid path
- `ScanError { path: PathBuf, message: String }` - Scanning error
- `ImportError(String)` - Data import error
- `ExportError(String)` - Data export error
- `ConfigError(String)` - Configuration error
- `UiError(String)` - UI/terminal error
- `ParseError(String)` - Parsing error
- `CompressionError(String)` - Compression error
- `ThreadError(String)` - Threading error
- `FileSystemError(String)` - File system error
- `UserCancelled` - Operation cancelled
- `FeatureNotAvailable(String)` - Feature not available
- `Internal(String)` - Internal error

#### Type Alias

```rust
pub type Result<T> = std::result::Result<T, RsduError>
```

#### Utility Functions

```rust
pub fn io_error_with_path<P: Into<PathBuf>>(error: io::Error, path: P) -> RsduError
```

#### Traits

```rust
pub trait ResultExt<T> {
    fn with_path<P: Into<PathBuf>>(self, path: P) -> Result<T>;
}
```

## Usage Examples

### Basic Library Usage

```rust
use rsdu::{Config, scanner, export};
use std::path::Path;

// Scan a directory
let config = Config::default();
let root = scanner::scan_directory(Path::new("/home/user"), &config)?;

// Export to JSON
let json = export::export_to_json_string(&root)?;
println!("{}", json);
```

### Custom Configuration

```rust
use rsdu::{Config, cli::Args};
use clap::Parser;

// Parse command line
let args = Args::parse();

// Create configuration
let config = Config::from_args(&args)?;

// Use configuration for scanning
let root = scanner::scan_directory(&path, &config)?;
```

### Error Handling

```rust
use rsdu::{RsduError, Result};

fn scan_with_error_handling(path: &Path) -> Result<()> {
    match scanner::scan_directory(path, &config) {
        Ok(root) => {
            println!("Scan completed: {} items", root.total_items());
            Ok(())
        }
        Err(RsduError::PermissionDenied { path, .. }) => {
            eprintln!("Permission denied: {}", path.display());
            Ok(()) // Continue with partial results
        }
        Err(e) => Err(e), // Propagate other errors
    }
}
```

## Thread Safety

- `Entry` uses `Arc<>` for shared ownership across threads
- `ScanStats` uses `AtomicU64` for lock-free statistics
- `HardlinkMap` requires external synchronization
- UI components are not thread-safe (single-threaded by design)

## Memory Management

- Entries use reference counting (`Arc<Entry>`) for efficient sharing
- Parent references use `Weak<>` to prevent cycles
- Large directory trees are handled efficiently through lazy evaluation
- Serialization creates temporary copies only when needed

## Performance Considerations

- Directory scanning designed for parallel execution
- Hardlink tracking uses efficient hash maps
- Statistics use atomic operations to avoid locking
- Memory usage scales linearly with directory size
- String operations optimized for terminal display

## Error Recovery

- Filesystem errors are captured per-entry rather than failing entire scan
- Permission denied errors allow partial results
- UI errors trigger graceful terminal cleanup
- Import/export errors preserve partial data when possible

## Platform Compatibility

- Core functionality works on all Rust-supported platforms
- Terminal UI uses `crossterm` for cross-platform support
- File system operations handle platform-specific edge cases
- Path handling properly supports Unicode and special characters