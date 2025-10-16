//! Data model for file system entries
//!
//! This module defines the core data structures used to represent
//! files, directories, and their metadata in the file system tree.

// use crate::error::{Result, RsduError}; // TODO: Will be used for error handling
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Unique identifier for entries (used for hardlink tracking)
pub type EntryId = u64;

/// Device identifier (simplified from st_dev)
pub type DeviceId = u32;

/// Inode number
pub type InodeId = u64;

/// Block size in bytes (typically 512 or 4096)
pub const BLOCK_SIZE: u64 = 512;

/// Entry type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// Regular directory
    Directory,
    /// Regular file
    File,
    /// Symbolic link
    Symlink,
    /// Hard link (multiple names for same inode)
    Hardlink,
    /// Special file (device, pipe, socket, etc.)
    Special,
    /// Error accessing entry
    Error,
    /// Excluded by pattern
    Excluded,
    /// Different filesystem
    OtherFs,
    /// Kernel filesystem (proc, sys, etc.)
    KernelFs,
}

impl EntryType {
    /// Whether this entry type represents a directory-like object
    pub fn is_directory(&self) -> bool {
        matches!(
            self,
            EntryType::Directory | EntryType::OtherFs | EntryType::KernelFs
        )
    }

    /// Whether this entry should be counted in statistics
    pub fn is_countable(&self) -> bool {
        !matches!(self, EntryType::Error | EntryType::Excluded)
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryType::Directory => write!(f, "DIR"),
            EntryType::File => write!(f, "FILE"),
            EntryType::Symlink => write!(f, "LINK"),
            EntryType::Hardlink => write!(f, "HARD"),
            EntryType::Special => write!(f, "SPEC"),
            EntryType::Error => write!(f, "ERR"),
            EntryType::Excluded => write!(f, "EXCL"),
            EntryType::OtherFs => write!(f, "OTFS"),
            EntryType::KernelFs => write!(f, "KERN"),
        }
    }
}

/// Extended metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedInfo {
    pub mtime: Option<DateTime<Utc>>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub mode: Option<u32>,
}

impl ExtendedInfo {
    pub fn new() -> Self {
        Self {
            mtime: None,
            uid: None,
            gid: None,
            mode: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.mtime.is_none() && self.uid.is_none() && self.gid.is_none() && self.mode.is_none()
    }
}

impl Default for ExtendedInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable entry structure for import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEntry {
    pub id: EntryId,
    pub entry_type: EntryType,
    pub name: String,
    pub size: u64,
    pub blocks: u64,
    pub device: DeviceId,
    pub inode: InodeId,
    pub nlink: u32,
    pub extended: Option<ExtendedInfo>,
    pub error: Option<String>,
    pub children: Vec<SerializableEntry>,
}

/// Core entry structure representing a file system object
#[derive(Debug, Clone)]
pub struct Entry {
    /// Unique identifier for this entry
    pub id: EntryId,
    /// Entry type
    pub entry_type: EntryType,
    /// File/directory name (without path)
    pub name: OsString,
    /// Size in bytes (apparent size)
    pub size: u64,
    /// Size in 512-byte blocks (disk usage)
    pub blocks: u64,
    /// Device this entry resides on
    pub device: DeviceId,
    /// Inode number
    pub inode: InodeId,
    /// Number of hard links to this inode
    pub nlink: u32,
    /// Extended information (optional)
    pub extended: Option<ExtendedInfo>,
    /// Error message if entry_type is Error
    pub error: Option<String>,
    /// Children (if directory)
    pub children: Vec<Arc<Entry>>,
    /// Parent entry (weak reference to avoid cycles)
    pub parent: Option<std::sync::Weak<Entry>>,
}

impl Entry {
    /// Create a new entry
    pub fn new(
        id: EntryId,
        entry_type: EntryType,
        name: OsString,
        size: u64,
        blocks: u64,
        device: DeviceId,
        inode: InodeId,
        nlink: u32,
    ) -> Self {
        Self {
            id,
            entry_type,
            name,
            size,
            blocks,
            device,
            inode,
            nlink,
            extended: None,
            error: None,
            children: Vec::new(),
            parent: None,
        }
    }

    /// Create an error entry
    pub fn error(id: EntryId, name: OsString, error: String) -> Self {
        Self {
            id,
            entry_type: EntryType::Error,
            name,
            size: 0,
            blocks: 0,
            device: 0,
            inode: 0,
            nlink: 0,
            extended: None,
            error: Some(error),
            children: Vec::new(),
            parent: None,
        }
    }

    /// Get the full path of this entry
    pub fn full_path(&self) -> PathBuf {
        let mut _components: Vec<&OsString> = Vec::new();

        // For now, just return the name since parent relationship needs more work
        // TODO: Implement proper parent traversal
        PathBuf::from(&self.name)
    }

    /// Get the name as a string (lossy conversion)
    pub fn name_str(&self) -> String {
        self.name.to_string_lossy().to_string()
    }

    /// Check if this entry has an error
    pub fn has_error(&self) -> bool {
        self.entry_type == EntryType::Error
    }

    /// Check if this entry has sub-errors (errors in children)
    pub fn has_sub_error(&self) -> bool {
        self.children
            .iter()
            .any(|child| child.has_error() || child.has_sub_error())
    }

    /// Add a child entry
    pub fn add_child(&mut self, child: Entry) -> Arc<Entry> {
        let child_arc = Arc::new(child);
        // TODO: Set up proper parent reference - this needs more careful design
        self.children.push(child_arc.clone());
        child_arc
    }

    /// Get total size including all children
    pub fn total_size(&self) -> u64 {
        self.size + self.children.iter().map(|c| c.total_size()).sum::<u64>()
    }

    /// Get total blocks including all children
    pub fn total_blocks(&self) -> u64 {
        self.blocks + self.children.iter().map(|c| c.total_blocks()).sum::<u64>()
    }

    /// Get total item count including all children
    pub fn total_items(&self) -> u64 {
        1 + self.children.iter().map(|c| c.total_items()).sum::<u64>()
    }

    /// Calculate shared size (hardlinks that exist outside this subtree)
    pub fn shared_size(&self, hardlink_map: &HardlinkMap) -> u64 {
        let mut shared = 0u64;

        if self.nlink > 1 {
            let key = HardlinkKey::new(self.device, self.inode);
            if let Some(info) = hardlink_map.get(&key) {
                // If this hardlink appears in multiple places, count as shared
                if info.total_links > info.links_in_tree {
                    shared += self.size;
                }
            }
        }

        shared
            + self
                .children
                .iter()
                .map(|c| c.shared_size(hardlink_map))
                .sum::<u64>()
    }

    /// Calculate shared blocks (hardlinks that exist outside this subtree)
    pub fn shared_blocks(&self, hardlink_map: &HardlinkMap) -> u64 {
        let mut shared = 0u64;

        if self.nlink > 1 {
            let key = HardlinkKey::new(self.device, self.inode);
            if let Some(info) = hardlink_map.get(&key) {
                if info.total_links > info.links_in_tree {
                    shared += self.blocks;
                }
            }
        }

        shared
            + self
                .children
                .iter()
                .map(|c| c.shared_blocks(hardlink_map))
                .sum::<u64>()
    }

    /// Sort children according to given criteria
    pub fn sort_children(&mut self, sort_col: SortColumn, sort_order: SortOrder, dirs_first: bool) {
        self.children.sort_by(|a, b| {
            use std::cmp::Ordering;

            // Directory-first sorting
            if dirs_first {
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
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::Size => a.total_size().cmp(&b.total_size()),
                SortColumn::Blocks => a.total_blocks().cmp(&b.total_blocks()),
                SortColumn::Items => a.total_items().cmp(&b.total_items()),
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

    /// Convert to serializable format
    pub fn to_serializable(&self) -> SerializableEntry {
        SerializableEntry {
            id: self.id,
            entry_type: self.entry_type,
            name: self.name.to_string_lossy().to_string(),
            size: self.size,
            blocks: self.blocks,
            device: self.device,
            inode: self.inode,
            nlink: self.nlink,
            extended: self.extended.clone(),
            error: self.error.clone(),
            children: self.children.iter().map(|c| c.to_serializable()).collect(),
        }
    }

    /// Create from serializable format
    pub fn from_serializable(serializable: SerializableEntry) -> Arc<Self> {
        let mut entry = Entry::new(
            serializable.id,
            serializable.entry_type,
            serializable.name.into(),
            serializable.size,
            serializable.blocks,
            serializable.device,
            serializable.inode,
            serializable.nlink,
        );
        entry.extended = serializable.extended;
        entry.error = serializable.error;

        // Convert children
        let children: Vec<Arc<Entry>> = serializable
            .children
            .into_iter()
            .map(Self::from_serializable)
            .collect();

        entry.children = children;
        Arc::new(entry)
    }
}

/// Sorting criteria
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Size,
    Blocks,
    Items,
    Mtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Key for hardlink tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HardlinkKey {
    pub device: DeviceId,
    pub inode: InodeId,
}

impl HardlinkKey {
    pub fn new(device: DeviceId, inode: InodeId) -> Self {
        Self { device, inode }
    }
}

/// Information about a hardlinked file
#[derive(Debug, Clone)]
pub struct HardlinkInfo {
    /// Total number of hard links to this inode
    pub total_links: u32,
    /// Number of links found in the current tree
    pub links_in_tree: u32,
    /// Size of the file
    pub size: u64,
    /// Blocks used by the file
    pub blocks: u64,
    /// First entry encountered with this inode
    pub first_entry: Arc<Entry>,
}

/// Map for tracking hardlinks
pub type HardlinkMap = HashMap<HardlinkKey, HardlinkInfo>;

/// Statistics about a scan
#[derive(Debug, Default)]
pub struct ScanStats {
    /// Total entries processed
    pub total_entries: AtomicU64,
    /// Total directories processed
    pub directories: AtomicU64,
    /// Total files processed
    pub files: AtomicU64,
    /// Total errors encountered
    pub errors: AtomicU64,
    /// Total size in bytes
    pub total_size: AtomicU64,
    /// Total blocks
    pub total_blocks: AtomicU64,
}

impl ScanStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_entries(&self) {
        self.total_entries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_directories(&self) {
        self.directories.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_files(&self) {
        self.files.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_size(&self, size: u64) {
        self.total_size.fetch_add(size, Ordering::Relaxed);
    }

    pub fn add_blocks(&self, blocks: u64) {
        self.total_blocks.fetch_add(blocks, Ordering::Relaxed);
    }

    pub fn get_total_entries(&self) -> u64 {
        self.total_entries.load(Ordering::Relaxed)
    }

    pub fn get_directories(&self) -> u64 {
        self.directories.load(Ordering::Relaxed)
    }

    pub fn get_files(&self) -> u64 {
        self.files.load(Ordering::Relaxed)
    }

    pub fn get_errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn get_total_size(&self) -> u64 {
        self.total_size.load(Ordering::Relaxed)
    }

    pub fn get_total_blocks(&self) -> u64 {
        self.total_blocks.load(Ordering::Relaxed)
    }
}

/// Global entry ID generator
static NEXT_ENTRY_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a new unique entry ID
pub fn generate_entry_id() -> EntryId {
    NEXT_ENTRY_ID.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_creation() {
        let entry = Entry::new(1, EntryType::File, "test.txt".into(), 1024, 2, 1, 12345, 1);

        assert_eq!(entry.id, 1);
        assert_eq!(entry.entry_type, EntryType::File);
        assert_eq!(entry.name_str(), "test.txt");
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.blocks, 2);
    }

    #[test]
    fn test_entry_type_directory_check() {
        assert!(EntryType::Directory.is_directory());
        assert!(EntryType::OtherFs.is_directory());
        assert!(!EntryType::File.is_directory());
    }

    #[test]
    fn test_error_entry() {
        let entry = Entry::error(1, "bad_file".into(), "Permission denied".to_string());
        assert_eq!(entry.entry_type, EntryType::Error);
        assert!(entry.has_error());
        assert_eq!(entry.error.as_ref().unwrap(), "Permission denied");
    }

    #[test]
    fn test_hardlink_key() {
        let key1 = HardlinkKey::new(1, 12345);
        let key2 = HardlinkKey::new(1, 12345);
        let key3 = HardlinkKey::new(2, 12345);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_scan_stats() {
        let stats = ScanStats::new();
        stats.increment_entries();
        stats.increment_files();
        stats.add_size(1024);

        assert_eq!(stats.get_total_entries(), 1);
        assert_eq!(stats.get_files(), 1);
        assert_eq!(stats.get_total_size(), 1024);
    }

    #[test]
    fn test_extended_info() {
        let mut ext = ExtendedInfo::new();
        assert!(ext.is_empty());

        ext.mtime = Some(Utc::now());
        assert!(!ext.is_empty());
    }
}
