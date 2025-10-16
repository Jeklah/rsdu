//! Utility functions for rsdu
//!
//! This module contains various helper functions and utilities used
//! throughout the application.

use crate::error::{Result, RsduError};
use humansize::{format_size, BINARY, DECIMAL};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Format file size in human-readable format
pub fn format_file_size(size: u64, use_si: bool) -> String {
    if use_si {
        format_size(size, DECIMAL)
    } else {
        format_size(size, BINARY)
    }
}

/// Format block count in human-readable format
pub fn format_blocks(blocks: u64, use_si: bool) -> String {
    format_file_size(blocks * 512, use_si)
}

/// Format percentage
pub fn format_percentage(part: u64, total: u64) -> String {
    if total == 0 {
        "0.0%".to_string()
    } else {
        let percentage = (part as f64 / total as f64) * 100.0;
        format!("{:.1}%", percentage)
    }
}

/// Format number with thousands separator
pub fn format_number_with_separator(num: u64, separator: &str) -> String {
    let num_str = num.to_string();
    let chars: Vec<char> = num_str.chars().collect();
    let mut result = String::new();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push_str(separator);
        }
        result.push(*ch);
    }

    result
}

/// Check if a filename should be considered "hidden"
pub fn is_hidden_file<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

/// Expand user home directory in path
pub fn expand_user_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();

    if let Some(path_str) = path.to_str() {
        if path_str.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                let home_path = Path::new(&home);
                if path_str == "~" {
                    return Ok(home_path.to_path_buf());
                } else if path_str.starts_with("~/") {
                    return Ok(home_path.join(&path_str[2..]));
                }
            }
        }
    }

    Ok(path.to_path_buf())
}

/// Convert SystemTime to timestamp
pub fn system_time_to_timestamp(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Create a progress bar character based on percentage
pub fn create_progress_bar(percentage: f64, width: usize, style: &str) -> String {
    let filled_width = (percentage * width as f64 / 100.0).round() as usize;
    let filled_width = filled_width.min(width);

    match style {
        "hash" => {
            let filled = "#".repeat(filled_width);
            let empty = " ".repeat(width - filled_width);
            format!("{}{}", filled, empty)
        }
        "half-block" => {
            let filled = "▌".repeat(filled_width);
            let empty = " ".repeat(width - filled_width);
            format!("{}{}", filled, empty)
        }
        "eighth-block" => {
            let filled = "▏".repeat(filled_width);
            let empty = " ".repeat(width - filled_width);
            format!("{}{}", filled, empty)
        }
        _ => {
            let filled = "█".repeat(filled_width);
            let empty = " ".repeat(width - filled_width);
            format!("{}{}", filled, empty)
        }
    }
}

/// Natural string comparison for file names
pub fn natural_compare(a: &str, b: &str) -> std::cmp::Ordering {
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
                    // Extract numbers and compare numerically
                    let a_num = extract_number(&mut a_chars);
                    let b_num = extract_number(&mut b_chars);

                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    // Regular character comparison
                    let a_next = a_chars.next().unwrap();
                    let b_next = b_chars.next().unwrap();

                    match a_next.cmp(&b_next) {
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
    let mut num_str = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            num_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    num_str.parse().unwrap_or(0)
}

/// Escape string for display in terminal
pub fn escape_for_display(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' => "\\t".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            c if c.is_control() => format!("\\x{:02x}", c as u8),
            c => c.to_string(),
        })
        .collect()
}

/// Get file name from path as string
pub fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get file extension from path
pub fn path_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_string())
}

/// Check if path matches a glob pattern
pub fn matches_glob_pattern(path: &str, pattern: &str) -> bool {
    match glob::Pattern::new(pattern) {
        Ok(glob_pattern) => glob_pattern.matches(path),
        Err(_) => false,
    }
}

/// Create a directory if it doesn't exist
pub fn ensure_directory_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| RsduError::Io(e))?;
    }
    Ok(())
}

/// Get the current working directory
pub fn current_dir() -> Result<PathBuf> {
    std::env::current_dir().map_err(|e| RsduError::Io(e))
}

/// Convert OsStr to String with lossy conversion
pub fn osstr_to_string(s: &OsStr) -> String {
    s.to_string_lossy().to_string()
}

/// Truncate string to fit within specified width
pub fn truncate_string(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_width.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Pad string to specified width
pub fn pad_string(s: &str, width: usize, right_align: bool) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        let padding = " ".repeat(width - len);
        if right_align {
            format!("{}{}", padding, s)
        } else {
            format!("{}{}", s, padding)
        }
    }
}

/// Check if the current process is running in a container
pub fn is_running_in_container() -> bool {
    // Check for common container indicators
    std::path::Path::new("/.dockerenv").exists()
        || std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
        || std::fs::read_to_string("/proc/1/cgroup")
            .map(|content| content.contains("docker") || content.contains("kubepods"))
            .unwrap_or(false)
}

/// Get terminal size
pub fn get_terminal_size() -> (usize, usize) {
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        (cols as usize, rows as usize)
    } else {
        (80, 24) // Default fallback
    }
}

/// Check if stderr is a TTY
pub fn stderr_is_tty() -> bool {
    atty::is(atty::Stream::Stderr)
}

/// Check if stdout is a TTY
pub fn stdout_is_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}

/// Get the number of CPU cores
pub fn get_cpu_count() -> usize {
    num_cpus::get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(1024, false), "1 KiB");
        assert_eq!(format_file_size(1000, true), "1 kB");
    }

    #[test]
    fn test_format_percentage() {
        assert_eq!(format_percentage(25, 100), "25.0%");
        assert_eq!(format_percentage(1, 3), "33.3%");
        assert_eq!(format_percentage(0, 0), "0.0%");
    }

    #[test]
    fn test_is_hidden_file() {
        assert!(is_hidden_file(".hidden"));
        assert!(is_hidden_file("/path/to/.hidden"));
        assert!(!is_hidden_file("visible"));
        assert!(!is_hidden_file("/path/to/visible"));
    }

    #[test]
    fn test_natural_compare() {
        use std::cmp::Ordering;

        assert_eq!(natural_compare("file1.txt", "file2.txt"), Ordering::Less);
        assert_eq!(
            natural_compare("file10.txt", "file2.txt"),
            Ordering::Greater
        );
        assert_eq!(natural_compare("file1.txt", "file1.txt"), Ordering::Equal);
    }

    #[test]
    fn test_create_progress_bar() {
        let bar = create_progress_bar(50.0, 10, "hash");
        assert_eq!(bar, "#####     ");

        let bar = create_progress_bar(100.0, 5, "hash");
        assert_eq!(bar, "#####");
    }

    #[test]
    fn test_format_number_with_separator() {
        assert_eq!(format_number_with_separator(1000, ","), "1,000");
        assert_eq!(format_number_with_separator(1234567, ","), "1,234,567");
        assert_eq!(format_number_with_separator(123, ","), "123");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 5), "hi");
    }

    #[test]
    fn test_pad_string() {
        assert_eq!(pad_string("hello", 10, false), "hello     ");
        assert_eq!(pad_string("hello", 10, true), "     hello");
        assert_eq!(pad_string("hello world", 5, false), "hello world");
    }

    #[test]
    fn test_escape_for_display() {
        assert_eq!(escape_for_display("hello\tworld\n"), "hello\\tworld\\n");
        assert_eq!(escape_for_display("normal"), "normal");
    }

    #[test]
    fn test_matches_glob_pattern() {
        assert!(matches_glob_pattern("test.txt", "*.txt"));
        assert!(matches_glob_pattern("test.log", "test.*"));
        assert!(!matches_glob_pattern("test.txt", "*.log"));
    }
}
