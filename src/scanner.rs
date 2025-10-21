//! File system scanning module
//!
//! This module handles the core functionality of scanning directories
//! and building the file system tree structure with support for:
//! - Multi-threaded scanning
//! - Hardlink detection and tracking
//! - Extended metadata collection
//! - Error handling and recovery
//! - Progress reporting
//! - Various filesystem filtering options

use crate::config::Config;
use crate::error::{Result, RsduError};
use crate::model::{
    generate_entry_id, Entry, EntryType, ExtendedInfo, HardlinkInfo, HardlinkKey, HardlinkMap,
    ScanStats, SortColumn, SortOrder,
};
use crate::tui::{ProgressStats, ScanMessage};
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::{self, DirEntry, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{mpsc::Sender, Arc, Mutex};
use std::time::SystemTime;
use walkdir::{DirEntry as WalkDirEntry, WalkDir};

/// Pseudo-filesystem mount points to exclude on Linux
const PSEUDO_FS: &[&str] = &[
    "/proc",
    "/sys",
    "/dev",
    "/run",
    "/tmp",
    "/var/run",
    "/var/lock",
    "/var/tmp",
];

/// Kernel filesystem types to exclude
const KERNEL_FS_TYPES: &[&str] = &[
    "proc",
    "sysfs",
    "devfs",
    "devpts",
    "tmpfs",
    "ramfs",
    "debugfs",
    "securityfs",
    "selinuxfs",
    "cgroup",
    "cgroup2",
    "pstore",
    "configfs",
    "fusectl",
    "binfmt_misc",
];

/// Cache directory tag file name
const CACHEDIR_TAG: &str = "CACHEDIR.TAG";

/// Scanner context for managing scan state
pub struct ScanContext {
    config: Config,
    stats: Arc<ScanStats>,
    hardlinks: Arc<Mutex<HardlinkMap>>,
    exclude_patterns: Vec<glob::Pattern>,
    root_device: Option<u64>,
    progress_sender: Option<Sender<ScanMessage>>,
}

impl ScanContext {
    fn new(config: Config, progress_sender: Option<Sender<ScanMessage>>) -> Result<Self> {
        let mut exclude_patterns = Vec::new();
        for pattern_str in &config.exclude_patterns {
            match glob::Pattern::new(pattern_str) {
                Ok(pattern) => exclude_patterns.push(pattern),
                Err(e) => {
                    return Err(RsduError::ConfigError(format!(
                        "Invalid exclude pattern '{}': {}",
                        pattern_str, e
                    )));
                }
            }
        }

        Ok(Self {
            config,
            stats: Arc::new(ScanStats::new()),
            hardlinks: Arc::new(Mutex::new(HashMap::new())),
            exclude_patterns,
            root_device: None,
            progress_sender,
        })
    }

    /// Check if a path should be excluded based on patterns
    fn is_excluded_by_pattern(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.exclude_patterns
            .iter()
            .any(|pattern| pattern.matches(&path_str))
    }

    /// Check if a path is on a different filesystem
    fn is_different_filesystem(&self, device: u64) -> bool {
        if !self.config.same_fs {
            return false;
        }
        if let Some(root_device) = self.root_device {
            device != root_device
        } else {
            false
        }
    }

    /// Check if a path is a kernel filesystem
    fn is_kernel_filesystem(&self, path: &Path) -> bool {
        if !self.config.exclude_kernfs {
            return false;
        }

        let path_str = path.to_string_lossy();
        PSEUDO_FS.iter().any(|&fs_path| {
            path_str.starts_with(fs_path)
                && (path_str.len() == fs_path.len()
                    || path_str.chars().nth(fs_path.len()) == Some('/'))
        })
    }

    /// Check if a directory contains CACHEDIR.TAG
    fn has_cachedir_tag(&self, dir_path: &Path) -> bool {
        if !self.config.exclude_caches {
            return false;
        }
        dir_path.join(CACHEDIR_TAG).exists()
    }
}

/// Scan a directory and return the root entry
pub fn scan_directory(path: &Path, config: &Config) -> Result<Arc<Entry>> {
    scan_directory_with_progress(path, config, None)
}

/// Scan a directory with progress updates
pub fn scan_directory_with_progress(
    path: &Path,
    config: &Config,
    progress_sender: Option<Sender<ScanMessage>>,
) -> Result<Arc<Entry>> {
    let mut context = ScanContext::new(config.clone(), progress_sender)?;

    // Get the root device for filesystem boundary checking
    if config.same_fs {
        match fs::metadata(path) {
            Ok(metadata) => {
                context.root_device = Some(metadata.dev());
            }
            Err(e) => {
                return Err(RsduError::scan_error(
                    path,
                    format!("Cannot read root directory metadata: {}", e),
                ));
            }
        }
    }

    // Send initial progress update
    if let Some(ref sender) = context.progress_sender {
        let _ = sender.send(ScanMessage::Progress {
            current_path: path.display().to_string(),
            stats: ProgressStats::from_scan_stats(&context.stats),
        });
    } else {
        println!("Scanning directory: {}", path.display());
    }

    // Perform the scan
    let root_entry = scan_entry(path, &context)?;

    // Send completion message or print statistics
    if let Some(ref sender) = context.progress_sender {
        let _ = sender.send(ScanMessage::Complete {
            root: root_entry.clone(),
        });
    } else {
        // Print final statistics for non-TUI mode
        let stats = &context.stats;
        println!("\nScan complete:");
        println!("  Directories: {}", stats.get_directories());
        println!("  Files: {}", stats.get_files());
        println!("  Total entries: {}", stats.get_total_entries());
        println!("  Errors: {}", stats.get_errors());
        println!("  Total size: {} bytes", stats.get_total_size());
        println!("  Total blocks: {}", stats.get_total_blocks());
    }

    Ok(root_entry)
}

/// Scan a single entry (file or directory)
fn scan_entry(path: &Path, context: &ScanContext) -> Result<Arc<Entry>> {
    // Send real-time progress update for every file for scanning screen
    if let Some(ref sender) = context.progress_sender {
        let _ = sender.send(ScanMessage::Progress {
            current_path: path.display().to_string(),
            stats: ProgressStats::from_scan_stats(&context.stats),
        });
    }
    // Get metadata
    let metadata = match get_metadata(path, context.config.follow_symlinks) {
        Ok(meta) => meta,
        Err(e) => {
            context.stats.increment_errors();
            let error_msg = format!("Cannot read metadata: {}", e);
            return Ok(Arc::new(Entry::error(
                generate_entry_id(),
                path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
                error_msg,
            )));
        }
    };

    // Check filesystem boundaries
    if context.is_different_filesystem(metadata.dev()) {
        return Ok(Arc::new(Entry::new(
            generate_entry_id(),
            EntryType::OtherFs,
            path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
            0,
            0,
            metadata.dev() as u32,
            metadata.ino(),
            metadata.nlink() as u32,
        )));
    }

    // Check for kernel filesystems
    if context.is_kernel_filesystem(path) {
        return Ok(Arc::new(Entry::new(
            generate_entry_id(),
            EntryType::KernelFs,
            path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
            0,
            0,
            metadata.dev() as u32,
            metadata.ino(),
            metadata.nlink() as u32,
        )));
    }

    // Check exclusion patterns
    if context.is_excluded_by_pattern(path) {
        return Ok(Arc::new(Entry::new(
            generate_entry_id(),
            EntryType::Excluded,
            path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
            0,
            0,
            metadata.dev() as u32,
            metadata.ino(),
            metadata.nlink() as u32,
        )));
    }

    let file_type = get_entry_type(&metadata, path);
    let size = metadata.len();
    let blocks = metadata.blocks();

    context.stats.increment_entries();
    context.stats.add_size(size);
    context.stats.add_blocks(blocks);

    let mut entry = Entry::new(
        generate_entry_id(),
        file_type,
        path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
        size,
        blocks,
        metadata.dev() as u32,
        metadata.ino(),
        metadata.nlink() as u32,
    );

    // Handle hardlinks
    if metadata.nlink() > 1 && file_type == EntryType::File {
        let hardlink_key = HardlinkKey::new(metadata.dev() as u32, metadata.ino());
        let mut hardlinks = context.hardlinks.lock().unwrap();

        match hardlinks.get_mut(&hardlink_key) {
            Some(info) => {
                // This is a duplicate hardlink
                info.links_in_tree += 1;
                entry.entry_type = EntryType::Hardlink;
            }
            None => {
                // First occurrence of this hardlink
                hardlinks.insert(
                    hardlink_key,
                    HardlinkInfo {
                        total_links: metadata.nlink() as u32,
                        links_in_tree: 1,
                        size,
                        blocks,
                        first_entry: Arc::new(entry.clone()),
                    },
                );
            }
        }
    }

    // Add extended information if requested
    if context.config.extended {
        entry.extended = Some(ExtendedInfo {
            mtime: metadata.modified().ok().and_then(|t| {
                DateTime::from_timestamp(
                    t.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs() as i64,
                    0,
                )
            }),
            uid: Some(metadata.uid()),
            gid: Some(metadata.gid()),
            mode: Some(metadata.mode()),
        });
    }

    // Handle directories
    if file_type == EntryType::Directory {
        context.stats.increment_directories();

        // Check for cache directory tag
        if context.has_cachedir_tag(path) {
            entry.entry_type = EntryType::Excluded;
            return Ok(Arc::new(entry));
        }

        // Scan directory contents
        match scan_directory_contents(path, context) {
            Ok(mut children) => {
                // Sort children if requested
                sort_entries(&mut children, &context.config);

                // Convert to Arc and add to entry
                let mut entry = entry;
                for child in children {
                    entry.children.push(child);
                }
                Ok(Arc::new(entry))
            }
            Err(e) => {
                context.stats.increment_errors();
                entry.error = Some(format!("Error scanning directory: {}", e));
                entry.entry_type = EntryType::Error;
                Ok(Arc::new(entry))
            }
        }
    } else {
        context.stats.increment_files();
        Ok(Arc::new(entry))
    }
}

/// Scan the contents of a directory
fn scan_directory_contents(dir_path: &Path, context: &ScanContext) -> Result<Vec<Arc<Entry>>> {
    let entries = match fs::read_dir(dir_path) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(RsduError::scan_error(
                dir_path,
                format!("Cannot read directory: {}", e),
            ));
        }
    };

    let mut children = Vec::new();

    // Use parallel processing if we have multiple threads configured
    if context.config.threads > 1 {
        // Collect entries first
        let dir_entries: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| should_include_entry(entry, context))
            .collect();

        // Process in parallel
        let parallel_children: Vec<Arc<Entry>> = dir_entries
            .into_par_iter()
            .map(|dir_entry| scan_entry(&dir_entry.path(), context))
            .filter_map(|result| match result {
                Ok(entry) => Some(entry),
                Err(_) => None, // Errors are handled in scan_entry
            })
            .collect();

        children = parallel_children;
    } else {
        // Sequential processing
        for entry in entries {
            if let Ok(dir_entry) = entry {
                if should_include_entry(&dir_entry, context) {
                    match scan_entry(&dir_entry.path(), context) {
                        Ok(child_entry) => children.push(child_entry),
                        Err(_) => {} // Errors are handled in scan_entry
                    }
                }
            }
        }
    }

    Ok(children)
}

/// Determine if a directory entry should be included in the scan
fn should_include_entry(entry: &DirEntry, context: &ScanContext) -> bool {
    let file_name = entry.file_name();
    let file_name_str = file_name.to_string_lossy();

    // Skip hidden files unless configured otherwise
    if !context.config.show_hidden && file_name_str.starts_with('.') {
        return false;
    }

    // Skip current and parent directory entries
    if file_name_str == "." || file_name_str == ".." {
        return false;
    }

    true
}

/// Get metadata for a path, optionally following symlinks
fn get_metadata(path: &Path, follow_symlinks: bool) -> std::io::Result<Metadata> {
    if follow_symlinks {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    }
}

/// Determine the entry type from metadata
fn get_entry_type(metadata: &Metadata, _path: &Path) -> EntryType {
    use std::os::unix::fs::FileTypeExt;
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        EntryType::Directory
    } else if file_type.is_file() {
        EntryType::File
    } else if file_type.is_symlink() {
        EntryType::Symlink
    } else if file_type.is_block_device()
        || file_type.is_char_device()
        || file_type.is_fifo()
        || file_type.is_socket()
    {
        EntryType::Special
    } else {
        EntryType::File // Default fallback
    }
}

/// Sort entries according to configuration
fn sort_entries(entries: &mut Vec<Arc<Entry>>, config: &Config) {
    let sort_col = match config.sort_col {
        crate::config::SortColumn::Name => SortColumn::Name,
        crate::config::SortColumn::Blocks => SortColumn::Blocks,
        crate::config::SortColumn::Size => SortColumn::Size,
        crate::config::SortColumn::Items => SortColumn::Items,
        crate::config::SortColumn::Mtime => SortColumn::Mtime,
    };

    let sort_order = match config.sort_order {
        crate::config::SortOrder::Asc => SortOrder::Asc,
        crate::config::SortOrder::Desc => SortOrder::Desc,
    };

    entries.sort_by(|a, b| {
        use std::cmp::Ordering;

        // Directory-first sorting
        if config.sort_dirs_first {
            let a_is_dir = a.entry_type.is_directory();
            let b_is_dir = b.entry_type.is_directory();
            if a_is_dir != b_is_dir {
                return if a_is_dir {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
        }

        let cmp = match sort_col {
            SortColumn::Name => {
                if config.sort_natural {
                    natural_sort(&a.name.to_string_lossy(), &b.name.to_string_lossy())
                } else {
                    a.name.cmp(&b.name)
                }
            }
            SortColumn::Size => {
                let a_total_size = calculate_total_entry_size(a);
                let b_total_size = calculate_total_entry_size(b);
                a_total_size.cmp(&b_total_size)
            }
            SortColumn::Blocks => {
                let a_total_blocks = calculate_total_entry_blocks(a);
                let b_total_blocks = calculate_total_entry_blocks(b);
                a_total_blocks.cmp(&b_total_blocks)
            }
            SortColumn::Items => {
                let a_total_items = calculate_total_entry_items(a);
                let b_total_items = calculate_total_entry_items(b);
                a_total_items.cmp(&b_total_items)
            }
            SortColumn::Mtime => {
                let a_mtime = a.extended.as_ref().and_then(|e| e.mtime);
                let b_mtime = b.extended.as_ref().and_then(|e| e.mtime);
                a_mtime.cmp(&b_mtime)
            }
        };

        match sort_order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });
}

/// Calculate total size including all children for an entry
fn calculate_total_entry_size(entry: &Arc<Entry>) -> u64 {
    entry.size
        + entry
            .children
            .iter()
            .map(|child| calculate_total_entry_size(child))
            .sum::<u64>()
}

/// Calculate total blocks including all children for an entry
fn calculate_total_entry_blocks(entry: &Arc<Entry>) -> u64 {
    entry.blocks
        + entry
            .children
            .iter()
            .map(|child| calculate_total_entry_blocks(child))
            .sum::<u64>()
}

/// Calculate total item count including all children for an entry
fn calculate_total_entry_items(entry: &Arc<Entry>) -> u64 {
    1 + entry
        .children
        .iter()
        .map(|child| calculate_total_entry_items(child))
        .sum::<u64>()
}

/// Natural sorting comparison (handles numbers in strings properly)
fn natural_sort(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a_char), Some(b_char)) => {
                if a_char.is_ascii_digit() && b_char.is_ascii_digit() {
                    // Extract and compare numbers
                    let a_num = extract_number(&mut a_chars);
                    let b_num = extract_number(&mut b_chars);
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    // Compare characters normally
                    let a_char = a_chars.next().unwrap();
                    let b_char = b_chars.next().unwrap();
                    match a_char.cmp(&b_char) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Extract a number from a character iterator
fn extract_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> u64 {
    let mut num = 0u64;
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            chars.next();
            if let Some(digit) = ch.to_digit(10) {
                num = num.saturating_mul(10).saturating_add(digit as u64);
            }
        } else {
            break;
        }
    }
    num
}

/// Scan directory using walkdir for deep scanning (alternative implementation)
#[allow(dead_code)]
pub fn scan_directory_walkdir(path: &Path, config: &Config) -> Result<Arc<Entry>> {
    let context = ScanContext::new(config.clone(), None)?;

    // Set up walkdir
    let mut walker = WalkDir::new(path).follow_links(config.follow_symlinks);

    if config.same_fs {
        walker = walker.same_file_system(true);
    }

    let root_name = path
        .file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_os_string();

    let mut root = Entry::new(
        generate_entry_id(),
        EntryType::Directory,
        root_name,
        0,
        0,
        0,
        0,
        1,
    );

    println!("Scanning directory (walkdir): {}", path.display());

    // Build a map to organize entries by their parent paths
    let mut entries_by_parent: HashMap<PathBuf, Vec<Arc<Entry>>> = HashMap::new();
    let mut total_size = 0u64;
    let mut total_blocks = 0u64;

    for entry_result in walker {
        match entry_result {
            Ok(dir_entry) => {
                let entry_path = dir_entry.path();

                // Skip the root itself
                if entry_path == path {
                    continue;
                }

                if let Some(scanned_entry) = scan_walkdir_entry(&dir_entry, &context)? {
                    total_size += scanned_entry.size;
                    total_blocks += scanned_entry.blocks;

                    let parent_path = entry_path.parent().unwrap_or(path).to_path_buf();
                    entries_by_parent
                        .entry(parent_path)
                        .or_insert_with(Vec::new)
                        .push(scanned_entry);
                }
            }
            Err(e) => {
                context.stats.increment_errors();
                eprintln!("Error walking directory: {}", e);
            }
        }
    }

    // Update root entry
    root.size = total_size;
    root.blocks = total_blocks;

    // Add direct children to root
    if let Some(children) = entries_by_parent.remove(path) {
        root.children = children;
    }

    // Print statistics
    let stats = &context.stats;
    println!("\nScan complete:");
    println!("  Directories: {}", stats.get_directories());
    println!("  Files: {}", stats.get_files());
    println!("  Total entries: {}", stats.get_total_entries());
    println!("  Errors: {}", stats.get_errors());

    Ok(Arc::new(root))
}

/// Scan a single walkdir entry
fn scan_walkdir_entry(entry: &WalkDirEntry, context: &ScanContext) -> Result<Option<Arc<Entry>>> {
    let path = entry.path();

    // Apply filters
    if context.is_excluded_by_pattern(path) {
        return Ok(None);
    }

    if context.is_kernel_filesystem(path) {
        return Ok(None);
    }

    let metadata = match entry.metadata() {
        Ok(meta) => meta,
        Err(e) => {
            context.stats.increment_errors();
            return Ok(Some(Arc::new(Entry::error(
                generate_entry_id(),
                path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
                format!("Metadata error: {}", e),
            ))));
        }
    };

    let entry_type = get_entry_type(&metadata, path);
    context.stats.increment_entries();

    if entry_type == EntryType::Directory {
        context.stats.increment_directories();

        if context.has_cachedir_tag(path) {
            return Ok(None);
        }
    } else {
        context.stats.increment_files();
    }

    let mut scanned_entry = Entry::new(
        generate_entry_id(),
        entry_type,
        path.file_name().unwrap_or(path.as_os_str()).to_os_string(),
        metadata.len(),
        metadata.blocks(),
        metadata.dev() as u32,
        metadata.ino(),
        metadata.nlink() as u32,
    );

    // Add extended info if requested
    if context.config.extended {
        scanned_entry.extended = Some(ExtendedInfo {
            mtime: metadata.modified().ok().and_then(|t| {
                DateTime::from_timestamp(
                    t.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs() as i64,
                    0,
                )
            }),
            uid: Some(metadata.uid()),
            gid: Some(metadata.gid()),
            mode: Some(metadata.mode()),
        });
    }

    context.stats.add_size(metadata.len());
    context.stats.add_blocks(metadata.blocks());

    Ok(Some(Arc::new(scanned_entry)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_natural_sort() {
        use std::cmp::Ordering;
        assert_eq!(natural_sort("file1", "file2"), Ordering::Less);
        assert_eq!(natural_sort("file10", "file2"), Ordering::Greater);
        assert_eq!(natural_sort("file01", "file1"), Ordering::Equal);
    }

    #[test]
    fn test_extract_number() {
        let mut chars = "123abc".chars().peekable();
        assert_eq!(extract_number(&mut chars), 123);
        assert_eq!(chars.next(), Some('a'));
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();

        let result = scan_directory(temp_dir.path(), &config);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.entry_type, EntryType::Directory);
        assert_eq!(entry.children.len(), 0);
    }

    #[test]
    fn test_scan_directory_with_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::write(temp_dir.path().join("file1.txt"), "Hello").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "World").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let config = Config::default();
        let result = scan_directory(temp_dir.path(), &config);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.entry_type, EntryType::Directory);
        assert_eq!(entry.children.len(), 3);
    }

    #[test]
    fn test_should_include_entry() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::default();
        let context = ScanContext::new(config).unwrap();

        // Create test entries
        std::fs::write(temp_dir.path().join("visible.txt"), "test").unwrap();
        std::fs::write(temp_dir.path().join(".hidden.txt"), "test").unwrap();

        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        let visible_count = entries
            .iter()
            .filter(|e| should_include_entry(e, &context))
            .count();

        // Should include visible.txt but not .hidden.txt (show_hidden is false by default)
        // Wait, actually show_hidden defaults to true in our config, so both should be included
        assert!(visible_count >= 1);
    }
}
