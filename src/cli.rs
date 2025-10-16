//! Command-line interface definitions and argument parsing

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rsdu")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A disk usage analyzer with a Ratatui interface")]
#[command(
    long_about = "rsdu is a fast disk usage analyzer with an interface made with Ratatui. It is designed to find space hogs on remote servers where you don't have an entire graphical setup available."
)]
pub struct Args {
    /// Directory to scan (defaults to current directory)
    pub directory: Option<PathBuf>,

    /// Import previously scanned directory from FILE
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub import_file: Option<String>,

    /// Export scanned directory to FILE in JSON format
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub export_json: Option<String>,

    /// Export scanned directory to FILE in binary format
    #[arg(short = 'O', long = "output-binary", value_name = "FILE")]
    pub export_binary: Option<String>,

    /// Stay on same filesystem
    #[arg(short = 'x', long = "one-file-system")]
    pub same_fs: bool,

    /// Cross filesystem boundaries
    #[arg(long = "cross-file-system")]
    pub cross_fs: bool,

    /// Show extended information (enables mtime, permissions, etc.)
    #[arg(short = 'e', long = "extended")]
    pub extended: bool,

    /// Disable extended information
    #[arg(long = "no-extended")]
    pub no_extended: bool,

    /// Follow symbolic links (excluding directories)
    #[arg(short = 'L', long = "follow-symlinks")]
    pub follow_symlinks: bool,

    /// Don't follow symbolic links
    #[arg(long = "no-follow-symlinks")]
    pub no_follow_symlinks: bool,

    /// Exclude files matching PATTERN
    #[arg(long = "exclude", value_name = "PATTERN", action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Exclude files matching patterns in FILE
    #[arg(short = 'X', long = "exclude-from", value_name = "FILE")]
    pub exclude_from: Option<PathBuf>,

    /// Exclude directories containing CACHEDIR.TAG
    #[arg(long = "exclude-caches")]
    pub exclude_caches: bool,

    /// Include directories containing CACHEDIR.TAG
    #[arg(long = "include-caches")]
    pub include_caches: bool,

    /// Exclude Linux pseudo filesystems (procfs, sysfs, cgroup, etc.)
    #[arg(long = "exclude-kernfs")]
    pub exclude_kernfs: bool,

    /// Include Linux pseudo filesystems
    #[arg(long = "include-kernfs")]
    pub include_kernfs: bool,

    /// Number of threads to use for scanning
    #[arg(short = 't', long = "threads", value_name = "NUM")]
    pub threads: Option<usize>,

    /// Use Zstandard compression for export
    #[arg(short = 'c', long = "compress")]
    pub compress: bool,

    /// Don't use compression for export
    #[arg(long = "no-compress")]
    pub no_compress: bool,

    /// Compression level (1-22)
    #[arg(long = "compress-level", value_name = "NUM")]
    pub compress_level: Option<u8>,

    /// Block size for binary export in KiB (4-16000)
    #[arg(long = "export-block-size", value_name = "KIB")]
    pub export_block_size: Option<u16>,

    /// UI mode during scanning
    #[arg(short = '0', long = "no-ui", help = "No UI during scan")]
    pub ui_none: bool,

    #[arg(short = '1', long = "line-ui", help = "Minimal line UI during scan")]
    pub ui_line: bool,

    #[arg(short = '2', long = "full-ui", help = "Full ncurses UI during scan")]
    pub ui_full: bool,

    /// Slow UI updates (2 second interval)
    #[arg(short = 'q', long = "slow-ui-updates")]
    pub slow_updates: bool,

    /// Fast UI updates (100ms interval)
    #[arg(long = "fast-ui-updates")]
    pub fast_updates: bool,

    /// Enable shell spawning feature
    #[arg(long = "enable-shell")]
    pub enable_shell: bool,

    /// Disable shell spawning feature
    #[arg(long = "disable-shell")]
    pub disable_shell: bool,

    /// Enable file deletion feature
    #[arg(long = "enable-delete")]
    pub enable_delete: bool,

    /// Disable file deletion feature
    #[arg(long = "disable-delete")]
    pub disable_delete: bool,

    /// Enable directory refresh feature
    #[arg(long = "enable-refresh")]
    pub enable_refresh: bool,

    /// Disable directory refresh feature
    #[arg(long = "disable-refresh")]
    pub disable_refresh: bool,

    /// Read-only mode (disable delete and shell)
    #[arg(short = 'r', long = "read-only")]
    pub read_only: bool,

    /// Use SI (base 10) prefixes instead of binary prefixes
    #[arg(long = "si")]
    pub si: bool,

    /// Use binary prefixes (default)
    #[arg(long = "no-si")]
    pub no_si: bool,

    /// Show apparent size instead of disk usage
    #[arg(long = "apparent-size")]
    pub apparent_size: bool,

    /// Show disk usage (default)
    #[arg(long = "disk-usage")]
    pub disk_usage: bool,

    /// Show hidden files by default
    #[arg(long = "show-hidden")]
    pub show_hidden: bool,

    /// Hide hidden files by default
    #[arg(long = "hide-hidden")]
    pub hide_hidden: bool,

    /// Show item count column by default
    #[arg(long = "show-itemcount")]
    pub show_itemcount: bool,

    /// Hide item count column by default
    #[arg(long = "hide-itemcount")]
    pub hide_itemcount: bool,

    /// Show modification time column by default (requires -e)
    #[arg(long = "show-mtime")]
    pub show_mtime: bool,

    /// Hide modification time column by default
    #[arg(long = "hide-mtime")]
    pub hide_mtime: bool,

    /// Show graph column by default
    #[arg(long = "show-graph")]
    pub show_graph: bool,

    /// Hide graph column by default
    #[arg(long = "hide-graph")]
    pub hide_graph: bool,

    /// Show percentage column by default
    #[arg(long = "show-percent")]
    pub show_percent: bool,

    /// Hide percentage column by default
    #[arg(long = "hide-percent")]
    pub hide_percent: bool,

    /// Graph style for usage bars
    #[arg(long = "graph-style", value_enum)]
    pub graph_style: Option<GraphStyle>,

    /// Shared column display mode
    #[arg(long = "shared-column", value_enum)]
    pub shared_column: Option<SharedColumn>,

    /// Sort column and order
    #[arg(long = "sort", value_name = "COLUMN")]
    pub sort: Option<String>,

    /// Use natural sorting for names
    #[arg(long = "enable-natsort")]
    pub enable_natsort: bool,

    /// Disable natural sorting for names
    #[arg(long = "disable-natsort")]
    pub disable_natsort: bool,

    /// Group directories before files
    #[arg(long = "group-directories-first")]
    pub group_directories_first: bool,

    /// Don't group directories before files
    #[arg(long = "no-group-directories-first")]
    pub no_group_directories_first: bool,

    /// Ask confirmation before quitting
    #[arg(long = "confirm-quit")]
    pub confirm_quit: bool,

    /// Don't ask confirmation before quitting
    #[arg(long = "no-confirm-quit")]
    pub no_confirm_quit: bool,

    /// Ask confirmation before deletion
    #[arg(long = "confirm-delete")]
    pub confirm_delete: bool,

    /// Don't ask confirmation before deletion
    #[arg(long = "no-confirm-delete")]
    pub no_confirm_delete: bool,

    /// Command to run for file deletion
    #[arg(long = "delete-command", value_name = "CMD")]
    pub delete_command: Option<String>,

    /// Color scheme
    #[arg(long = "color", value_enum)]
    pub color: Option<ColorScheme>,

    /// Don't load configuration files
    #[arg(long = "ignore-config")]
    pub ignore_config: bool,
}

#[derive(ValueEnum, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum GraphStyle {
    Hash,
    #[value(name = "half-block")]
    HalfBlock,
    #[value(name = "eighth-block")]
    EighthBlock,
}

#[derive(ValueEnum, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SharedColumn {
    Off,
    Shared,
    Unique,
}

#[derive(ValueEnum, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ColorScheme {
    Off,
    Dark,
    #[value(name = "dark-bg")]
    DarkBg,
}

impl Args {
    /// Validate arguments for consistency
    pub fn validate(&self) -> Result<(), String> {
        // Check for conflicting options
        if self.ui_none as u8 + self.ui_line as u8 + self.ui_full as u8 > 1 {
            return Err("Only one UI mode can be specified".to_string());
        }

        if self.same_fs && self.cross_fs {
            return Err(
                "--one-file-system and --cross-file-system are mutually exclusive".to_string(),
            );
        }

        if self.extended && self.no_extended {
            return Err("--extended and --no-extended are mutually exclusive".to_string());
        }

        if self.follow_symlinks && self.no_follow_symlinks {
            return Err(
                "--follow-symlinks and --no-follow-symlinks are mutually exclusive".to_string(),
            );
        }

        if self.exclude_caches && self.include_caches {
            return Err("--exclude-caches and --include-caches are mutually exclusive".to_string());
        }

        if self.exclude_kernfs && self.include_kernfs {
            return Err("--exclude-kernfs and --include-kernfs are mutually exclusive".to_string());
        }

        if self.compress && self.no_compress {
            return Err("--compress and --no-compress are mutually exclusive".to_string());
        }

        if self.si && self.no_si {
            return Err("--si and --no-si are mutually exclusive".to_string());
        }

        if self.apparent_size && self.disk_usage {
            return Err("--apparent-size and --disk-usage are mutually exclusive".to_string());
        }

        if self.show_hidden && self.hide_hidden {
            return Err("--show-hidden and --hide-hidden are mutually exclusive".to_string());
        }

        if self.enable_natsort && self.disable_natsort {
            return Err(
                "--enable-natsort and --disable-natsort are mutually exclusive".to_string(),
            );
        }

        if self.group_directories_first && self.no_group_directories_first {
            return Err(
                "--group-directories-first and --no-group-directories-first are mutually exclusive"
                    .to_string(),
            );
        }

        if self.confirm_quit && self.no_confirm_quit {
            return Err("--confirm-quit and --no-confirm-quit are mutually exclusive".to_string());
        }

        if self.confirm_delete && self.no_confirm_delete {
            return Err(
                "--confirm-delete and --no-confirm-delete are mutually exclusive".to_string(),
            );
        }

        // Validate numeric ranges
        if let Some(threads) = self.threads {
            if threads == 0 {
                return Err("Number of threads must be greater than 0".to_string());
            }
        }

        if let Some(level) = self.compress_level {
            if !(1..=22).contains(&level) {
                return Err("Compression level must be between 1 and 22".to_string());
            }
        }

        if let Some(block_size) = self.export_block_size {
            if !(4..=16000).contains(&block_size) {
                return Err("Export block size must be between 4 and 16000 KiB".to_string());
            }
        }

        // Validate sort option format
        if let Some(sort) = &self.sort {
            if !is_valid_sort_option(sort) {
                return Err(format!("Invalid sort option: {}", sort));
            }
        }

        Ok(())
    }
}

fn is_valid_sort_option(sort: &str) -> bool {
    let valid_columns = ["name", "disk-usage", "apparent-size", "itemcount", "mtime"];
    let valid_orders = ["asc", "desc"];

    if let Some((column, order)) = sort.rsplit_once('-') {
        valid_columns.contains(&column) && valid_orders.contains(&order)
    } else {
        valid_columns.contains(&sort)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_validation() {
        assert!(is_valid_sort_option("name"));
        assert!(is_valid_sort_option("name-asc"));
        assert!(is_valid_sort_option("disk-usage-desc"));
        assert!(!is_valid_sort_option("invalid"));
        assert!(!is_valid_sort_option("name-invalid"));
    }

    #[test]
    fn test_args_validation() {
        let mut args = Args {
            directory: None,
            import_file: None,
            export_json: None,
            export_binary: None,
            same_fs: false,
            cross_fs: false,
            extended: false,
            no_extended: false,
            follow_symlinks: false,
            no_follow_symlinks: false,
            exclude: Vec::new(),
            exclude_from: None,
            exclude_caches: false,
            include_caches: false,
            exclude_kernfs: false,
            include_kernfs: false,
            threads: None,
            compress: false,
            no_compress: false,
            compress_level: None,
            export_block_size: None,
            ui_none: false,
            ui_line: false,
            ui_full: false,
            slow_updates: false,
            fast_updates: false,
            enable_shell: false,
            disable_shell: false,
            enable_delete: false,
            disable_delete: false,
            enable_refresh: false,
            disable_refresh: false,
            read_only: false,
            si: false,
            no_si: false,
            apparent_size: false,
            disk_usage: false,
            show_hidden: false,
            hide_hidden: false,
            show_itemcount: false,
            hide_itemcount: false,
            show_mtime: false,
            hide_mtime: false,
            show_graph: false,
            hide_graph: false,
            show_percent: false,
            hide_percent: false,
            graph_style: None,
            shared_column: None,
            sort: None,
            enable_natsort: false,
            disable_natsort: false,
            group_directories_first: false,
            no_group_directories_first: false,
            confirm_quit: false,
            no_confirm_quit: false,
            confirm_delete: false,
            no_confirm_delete: false,
            delete_command: None,
            color: None,
            ignore_config: false,
        };

        // Valid args should pass
        assert!(args.validate().is_ok());

        // Conflicting args should fail
        args.same_fs = true;
        args.cross_fs = true;
        assert!(args.validate().is_err());
    }
}
