//! Configuration management for rsdu
//!
//! This module handles configuration loading from command line arguments,
//! configuration files, and environment variables.

use crate::cli::{Args, ColorScheme, GraphStyle, SharedColumn};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
// use std::collections::HashSet; // TODO: Will be used for pattern matching
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Scan options
    pub same_fs: bool,
    pub extended: bool,
    pub follow_symlinks: bool,
    pub exclude_caches: bool,
    pub exclude_kernfs: bool,
    pub threads: usize,
    pub exclude_patterns: Vec<String>,

    // Export/Import options
    pub compress: bool,
    pub compress_level: u8,
    pub export_block_size: Option<usize>,
    pub export_json: Option<String>,
    pub export_binary: Option<String>,

    // UI options
    pub scan_ui: Option<ScanUi>,
    pub update_delay: Duration,
    pub si: bool,
    pub color: ColorScheme,

    // Display options
    pub show_hidden: bool,
    pub show_blocks: bool, // true for disk usage, false for apparent size
    pub show_shared: SharedColumn,
    pub show_items: bool,
    pub show_mtime: bool,
    pub show_graph: bool,
    pub show_percent: bool,
    pub graph_style: GraphStyle,

    // Sorting options
    pub sort_col: SortColumn,
    pub sort_order: SortOrder,
    pub sort_dirs_first: bool,
    pub sort_natural: bool,

    // Feature flags
    pub can_delete: Option<bool>,
    pub can_shell: Option<bool>,
    pub can_refresh: Option<bool>,
    pub confirm_quit: bool,
    pub confirm_delete: bool,
    pub delete_command: String,

    // Internal flags
    pub imported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanUi {
    None,
    Line,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortColumn {
    Name,
    Blocks,
    Size,
    Items,
    Mtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Scan options
            same_fs: false,
            extended: false,
            follow_symlinks: false,
            exclude_caches: false,
            exclude_kernfs: false,
            threads: num_cpus::get().max(1),
            exclude_patterns: Vec::new(),

            // Export/Import options
            compress: false,
            compress_level: 4,
            export_block_size: None,
            export_json: None,
            export_binary: None,

            // UI options
            scan_ui: None,
            update_delay: Duration::from_millis(100),
            si: false,
            color: ColorScheme::Off,

            // Display options
            show_hidden: true,
            show_blocks: true,
            show_shared: SharedColumn::Shared,
            show_items: false,
            show_mtime: false,
            show_graph: true,
            show_percent: false,
            graph_style: GraphStyle::Hash,

            // Sorting options
            sort_col: SortColumn::Size,
            sort_order: SortOrder::Desc,
            sort_dirs_first: false,
            sort_natural: true,

            // Feature flags
            can_delete: None,
            can_shell: None,
            can_refresh: None,
            confirm_quit: false,
            confirm_delete: true,
            delete_command: String::new(),

            // Internal flags
            imported: false,
        }
    }
}

impl Config {
    /// Create configuration from command line arguments
    pub fn from_args(args: &Args) -> Result<Self> {
        // Validate arguments first
        args.validate()
            .map_err(|e| anyhow::anyhow!("Invalid command line arguments: {}", e))?;

        let mut config = if args.ignore_config {
            Self::default()
        } else {
            Self::load_from_files()?
        };

        // Apply command line arguments (they override config files)
        config.apply_args(args)?;

        // Set default threads if not specified
        if config.threads == 0 {
            config.threads = num_cpus::get().max(1);
        }

        Ok(config)
    }

    /// Load configuration from standard config file locations
    fn load_from_files() -> Result<Self> {
        let mut config = Self::default();

        // Try to load from system config
        if let Ok(system_config) = Self::load_config_file("/etc/rsdu.conf") {
            config.merge(system_config);
        }

        // Try to load from user config
        if let Some(config_dir) = get_user_config_dir() {
            let user_config_path = config_dir.join("rsdu").join("config");
            if let Ok(user_config) = Self::load_config_file(&user_config_path) {
                config.merge(user_config);
            }
        }

        Ok(config)
    }

    /// Load configuration from a specific file
    fn load_config_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        // Simple key=value parser for config files
        Self::parse_config_content(&content)
    }

    /// Parse configuration content from a string
    fn parse_config_content(content: &str) -> Result<Self> {
        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle @option syntax for error-tolerant parsing
            let (line, ignore_error) = if line.starts_with('@') {
                (&line[1..], true)
            } else {
                (line, false)
            };

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                if let Err(e) = config.apply_config_option(key, value) {
                    if !ignore_error {
                        return Err(e).with_context(|| format!("Error in config line: {}", line));
                    }
                }
            } else if let Err(e) = config.apply_config_flag(line) {
                if !ignore_error {
                    return Err(e).with_context(|| format!("Error in config line: {}", line));
                }
            }
        }

        Ok(config)
    }

    /// Apply a configuration flag (boolean option)
    fn apply_config_flag(&mut self, flag: &str) -> Result<()> {
        match flag {
            "same-fs" | "one-file-system" => self.same_fs = true,
            "cross-file-system" => self.same_fs = false,
            "extended" => self.extended = true,
            "no-extended" => self.extended = false,
            "follow-symlinks" => self.follow_symlinks = true,
            "no-follow-symlinks" => self.follow_symlinks = false,
            "exclude-caches" => self.exclude_caches = true,
            "include-caches" => self.exclude_caches = false,
            "exclude-kernfs" => self.exclude_kernfs = true,
            "include-kernfs" => self.exclude_kernfs = false,
            "compress" => self.compress = true,
            "no-compress" => self.compress = false,
            "si" => self.si = true,
            "no-si" => self.si = false,
            "show-hidden" => self.show_hidden = true,
            "hide-hidden" => self.show_hidden = false,
            "apparent-size" => self.show_blocks = false,
            "disk-usage" => self.show_blocks = true,
            "show-itemcount" => self.show_items = true,
            "hide-itemcount" => self.show_items = false,
            "show-mtime" => self.show_mtime = true,
            "hide-mtime" => self.show_mtime = false,
            "show-graph" => self.show_graph = true,
            "hide-graph" => self.show_graph = false,
            "show-percent" => self.show_percent = true,
            "hide-percent" => self.show_percent = false,
            "group-directories-first" => self.sort_dirs_first = true,
            "no-group-directories-first" => self.sort_dirs_first = false,
            "enable-natsort" => self.sort_natural = true,
            "disable-natsort" => self.sort_natural = false,
            "confirm-quit" => self.confirm_quit = true,
            "no-confirm-quit" => self.confirm_quit = false,
            "confirm-delete" => self.confirm_delete = true,
            "no-confirm-delete" => self.confirm_delete = false,
            "enable-shell" => self.can_shell = Some(true),
            "disable-shell" => self.can_shell = Some(false),
            "enable-delete" => self.can_delete = Some(true),
            "disable-delete" => self.can_delete = Some(false),
            "enable-refresh" => self.can_refresh = Some(true),
            "disable-refresh" => self.can_refresh = Some(false),
            _ => return Err(anyhow::anyhow!("Unknown config flag: {}", flag)),
        }
        Ok(())
    }

    /// Apply a configuration key-value option
    fn apply_config_option(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "threads" => self.threads = value.parse()?,
            "compress-level" => self.compress_level = value.parse()?,
            "export-block-size" => {
                let size: u16 = value.parse()?;
                self.export_block_size = Some(size as usize * 1024);
            }
            "exclude" => self.exclude_patterns.push(value.to_string()),
            "delete-command" => self.delete_command = value.to_string(),
            "extended" => {
                self.extended = match value {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Invalid boolean value for extended: {}",
                            value
                        ))
                    }
                };
            }
            "color" => {
                self.color = match value {
                    "off" => ColorScheme::Off,
                    "dark" => ColorScheme::Dark,
                    "dark-bg" => ColorScheme::DarkBg,
                    _ => return Err(anyhow::anyhow!("Invalid color scheme: {}", value)),
                };
            }
            "graph-style" => {
                self.graph_style = match value {
                    "hash" => GraphStyle::Hash,
                    "half-block" => GraphStyle::HalfBlock,
                    "eighth-block" => GraphStyle::EighthBlock,
                    _ => return Err(anyhow::anyhow!("Invalid graph style: {}", value)),
                };
            }
            "shared-column" => {
                self.show_shared = match value {
                    "off" => SharedColumn::Off,
                    "shared" => SharedColumn::Shared,
                    "unique" => SharedColumn::Unique,
                    _ => return Err(anyhow::anyhow!("Invalid shared column mode: {}", value)),
                };
            }
            "sort" => self.parse_sort_option(value)?,
            _ => return Err(anyhow::anyhow!("Unknown config option: {}", key)),
        }
        Ok(())
    }

    /// Parse sort option string
    fn parse_sort_option(&mut self, sort: &str) -> Result<()> {
        let (column, order) = if let Some((col, ord)) = sort.rsplit_once('-') {
            (col, Some(ord))
        } else {
            (sort, None)
        };

        self.sort_col = match column {
            "name" => SortColumn::Name,
            "disk-usage" => SortColumn::Blocks,
            "blocks" => SortColumn::Blocks,
            "apparent-size" => SortColumn::Size,
            "itemcount" => SortColumn::Items,
            "mtime" => SortColumn::Mtime,
            _ => return Err(anyhow::anyhow!("Invalid sort column: {}", column)),
        };

        if let Some(order) = order {
            self.sort_order = match order {
                "asc" => SortOrder::Asc,
                "desc" => SortOrder::Desc,
                _ => return Err(anyhow::anyhow!("Invalid sort order: {}", order)),
            };
        } else {
            // Set default order based on column
            self.sort_order = match self.sort_col {
                SortColumn::Name | SortColumn::Mtime => SortOrder::Asc,
                SortColumn::Blocks | SortColumn::Size | SortColumn::Items => SortOrder::Desc,
            };
        }

        Ok(())
    }

    /// Apply command line arguments to override config
    fn apply_args(&mut self, args: &Args) -> Result<()> {
        // Scan options
        if args.same_fs {
            self.same_fs = true;
        }
        if args.cross_fs {
            self.same_fs = false;
        }
        if args.extended {
            self.extended = true;
        }
        if args.no_extended {
            self.extended = false;
        }
        if args.follow_symlinks {
            self.follow_symlinks = true;
        }
        if args.no_follow_symlinks {
            self.follow_symlinks = false;
        }
        if args.exclude_caches {
            self.exclude_caches = true;
        }
        if args.include_caches {
            self.exclude_caches = false;
        }
        if args.exclude_kernfs {
            self.exclude_kernfs = true;
        }
        if args.include_kernfs {
            self.exclude_kernfs = false;
        }

        if let Some(threads) = args.threads {
            self.threads = threads;
        }

        // Add exclude patterns
        for pattern in &args.exclude {
            self.exclude_patterns.push(pattern.clone());
        }

        // Load exclude patterns from file
        if let Some(exclude_file) = &args.exclude_from {
            self.load_exclude_file(exclude_file)?;
        }

        // Export options
        self.export_json = args.export_json.clone();
        self.export_binary = args.export_binary.clone();

        if args.compress {
            self.compress = true;
        }
        if args.no_compress {
            self.compress = false;
        }

        if let Some(level) = args.compress_level {
            self.compress_level = level;
        }

        if let Some(block_size) = args.export_block_size {
            self.export_block_size = Some(block_size as usize * 1024);
        }

        // UI options
        if args.ui_none {
            self.scan_ui = Some(ScanUi::None);
        }
        if args.ui_line {
            self.scan_ui = Some(ScanUi::Line);
        }
        if args.ui_full {
            self.scan_ui = Some(ScanUi::Full);
        }

        if args.slow_updates {
            self.update_delay = Duration::from_secs(2);
        }
        if args.fast_updates {
            self.update_delay = Duration::from_millis(100);
        }

        if args.si {
            self.si = true;
        }
        if args.no_si {
            self.si = false;
        }

        // Display options
        if args.show_hidden {
            self.show_hidden = true;
        }
        if args.hide_hidden {
            self.show_hidden = false;
        }
        if args.apparent_size {
            self.show_blocks = false;
        }
        if args.disk_usage {
            self.show_blocks = true;
        }
        if args.show_itemcount {
            self.show_items = true;
        }
        if args.hide_itemcount {
            self.show_items = false;
        }
        if args.show_mtime {
            self.show_mtime = true;
        }
        if args.hide_mtime {
            self.show_mtime = false;
        }
        if args.show_graph {
            self.show_graph = true;
        }
        if args.hide_graph {
            self.show_graph = false;
        }
        if args.show_percent {
            self.show_percent = true;
        }
        if args.hide_percent {
            self.show_percent = false;
        }

        if let Some(style) = &args.graph_style {
            self.graph_style = style.clone();
        }
        if let Some(shared) = &args.shared_column {
            self.show_shared = shared.clone();
        }

        // Sorting options
        if let Some(sort) = &args.sort {
            self.parse_sort_option(sort)?;
        }

        if args.enable_natsort {
            self.sort_natural = true;
        }
        if args.disable_natsort {
            self.sort_natural = false;
        }
        if args.group_directories_first {
            self.sort_dirs_first = true;
        }
        if args.no_group_directories_first {
            self.sort_dirs_first = false;
        }

        // Feature flags
        if args.enable_shell {
            self.can_shell = Some(true);
        }
        if args.disable_shell {
            self.can_shell = Some(false);
        }
        if args.enable_delete {
            self.can_delete = Some(true);
        }
        if args.disable_delete {
            self.can_delete = Some(false);
        }
        if args.enable_refresh {
            self.can_refresh = Some(true);
        }
        if args.disable_refresh {
            self.can_refresh = Some(false);
        }
        if args.read_only {
            self.can_delete = Some(false);
            self.can_shell = Some(false);
        }

        if args.confirm_quit {
            self.confirm_quit = true;
        }
        if args.no_confirm_quit {
            self.confirm_quit = false;
        }
        if args.confirm_delete {
            self.confirm_delete = true;
        }
        if args.no_confirm_delete {
            self.confirm_delete = false;
        }

        if let Some(cmd) = &args.delete_command {
            self.delete_command = cmd.clone();
        }

        if let Some(color) = &args.color {
            self.color = color.clone();
        }

        Ok(())
    }

    /// Load exclude patterns from a file
    fn load_exclude_file(&mut self, path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read exclude file: {}", path.display()))?;

        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                self.exclude_patterns.push(line.to_string());
            }
        }

        Ok(())
    }

    /// Merge another configuration into this one
    fn merge(&mut self, other: Self) {
        // This is a simple merge - we could make it more sophisticated
        // For now, just take non-default values from other
        if other.same_fs {
            self.same_fs = true;
        }
        if other.extended {
            self.extended = true;
        }
        if other.follow_symlinks {
            self.follow_symlinks = true;
        }
        if other.exclude_caches {
            self.exclude_caches = true;
        }
        if other.exclude_kernfs {
            self.exclude_kernfs = true;
        }
        if other.threads != num_cpus::get().max(1) {
            self.threads = other.threads;
        }
        self.exclude_patterns.extend(other.exclude_patterns);

        if other.compress {
            self.compress = true;
        }
        if other.compress_level != 4 {
            self.compress_level = other.compress_level;
        }
        if other.export_block_size.is_some() {
            self.export_block_size = other.export_block_size;
        }

        if other.scan_ui.is_some() {
            self.scan_ui = other.scan_ui;
        }
        if other.update_delay != Duration::from_millis(100) {
            self.update_delay = other.update_delay;
        }
        if other.si {
            self.si = true;
        }

        // Display options
        if !other.show_hidden {
            self.show_hidden = false;
        }
        if !other.show_blocks {
            self.show_blocks = false;
        }
        if other.show_items {
            self.show_items = true;
        }
        if other.show_mtime {
            self.show_mtime = true;
        }
        if !other.show_graph {
            self.show_graph = false;
        }
        if other.show_percent {
            self.show_percent = true;
        }

        // Feature flags
        if other.can_delete.is_some() {
            self.can_delete = other.can_delete;
        }
        if other.can_shell.is_some() {
            self.can_shell = other.can_shell;
        }
        if other.can_refresh.is_some() {
            self.can_refresh = other.can_refresh;
        }
        if other.confirm_quit {
            self.confirm_quit = true;
        }
        if !other.confirm_delete {
            self.confirm_delete = false;
        }
        if !other.delete_command.is_empty() {
            self.delete_command = other.delete_command;
        }
    }
}

/// Get the user's configuration directory
fn get_user_config_dir() -> Option<PathBuf> {
    // First try XDG_CONFIG_HOME
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config));
    }

    // Fall back to ~/.config
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".config"));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.same_fs);
        assert!(!config.extended);
        assert!(config.threads > 0);
    }

    #[test]
    fn test_config_parsing() {
        let content = r#"
# This is a comment
same-fs
threads=8
exclude=*.tmp
"#;

        let config = Config::parse_config_content(content).unwrap();
        assert!(config.same_fs);
        assert_eq!(config.threads, 8);
        assert_eq!(config.exclude_patterns, vec!["*.tmp"]);
    }

    #[test]
    fn test_sort_parsing() {
        let mut config = Config::default();

        config.parse_sort_option("name-asc").unwrap();
        assert_eq!(config.sort_col, SortColumn::Name);
        assert_eq!(config.sort_order, SortOrder::Asc);

        config.parse_sort_option("blocks").unwrap();
        assert_eq!(config.sort_col, SortColumn::Blocks);
        assert_eq!(config.sort_order, SortOrder::Desc);
    }
}
