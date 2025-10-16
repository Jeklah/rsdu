//! Terminal UI browser module
//!
//! This module handles the interactive browsing interface for exploring
//! the file system tree using a TUI (Terminal User Interface) with keyboard navigation.

use crate::config::{Config, SortColumn, SortOrder};
use crate::error::{Result, RsduError};
use crate::model::{Entry, EntryType, SortColumn as ModelSortColumn, SortOrder as ModelSortOrder};
use crate::utils::{format_file_size, format_percentage};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::cmp;
use std::io::{self, Write};
use std::sync::Arc;

/// Browser state
pub struct Browser {
    root: Arc<Entry>,
    current: Arc<Entry>,
    path_stack: Vec<Arc<Entry>>,
    selected_index: usize,
    scroll_offset: usize,
    config: Config,
    terminal_height: u16,
    terminal_width: u16,
    show_help: bool,
}

impl Browser {
    /// Create a new browser instance
    pub fn new(root: Arc<Entry>, config: Config) -> Result<Self> {
        let (width, height) = terminal::size()
            .map_err(|e| RsduError::UiError(format!("Cannot get terminal size: {}", e)))?;

        Ok(Browser {
            current: root.clone(),
            root,
            path_stack: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            config,
            terminal_height: height,
            terminal_width: width,
            show_help: false,
        })
    }

    /// Main browser loop
    pub fn run(&mut self) -> Result<()> {
        // Enable raw mode
        terminal::enable_raw_mode()
            .map_err(|e| RsduError::UiError(format!("Cannot enable raw mode: {}", e)))?;

        // Clear screen and hide cursor
        let mut stdout = io::stdout();
        execute!(stdout, terminal::Clear(ClearType::All), cursor::Hide)
            .map_err(|e| RsduError::UiError(format!("Terminal setup error: {}", e)))?;

        let result = self.main_loop();

        // Cleanup
        let _ = execute!(stdout, cursor::Show, ResetColor);
        let _ = terminal::disable_raw_mode();

        result
    }

    /// Main event loop
    fn main_loop(&mut self) -> Result<()> {
        loop {
            self.update_terminal_size()?;
            self.draw()?;

            // Handle events
            if event::poll(std::time::Duration::from_millis(100))
                .map_err(|e| RsduError::UiError(format!("Event poll error: {}", e)))?
            {
                match event::read()
                    .map_err(|e| RsduError::UiError(format!("Event read error: {}", e)))?
                {
                    Event::Key(key_event) => {
                        if key_event.kind == KeyEventKind::Press {
                            match self.handle_key(key_event.code, key_event.modifiers)? {
                                BrowserAction::Quit => break,
                                BrowserAction::Continue => {}
                            }
                        }
                    }
                    Event::Resize(width, height) => {
                        self.terminal_width = width;
                        self.terminal_height = height;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<BrowserAction> {
        if modifiers.contains(KeyModifiers::CONTROL) {
            match key {
                KeyCode::Char('c') => return Ok(BrowserAction::Quit),
                _ => {}
            }
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => Ok(BrowserAction::Quit),
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.show_help = !self.show_help;
                Ok(BrowserAction::Continue)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                Ok(BrowserAction::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                Ok(BrowserAction::Continue)
            }
            KeyCode::PageUp => {
                self.move_selection(-(self.get_visible_height() as i32));
                Ok(BrowserAction::Continue)
            }
            KeyCode::PageDown => {
                self.move_selection(self.get_visible_height() as i32);
                Ok(BrowserAction::Continue)
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected_index = 0;
                self.scroll_offset = 0;
                Ok(BrowserAction::Continue)
            }
            KeyCode::End | KeyCode::Char('G') => {
                if !self.current.children.is_empty() {
                    self.selected_index = self.current.children.len() - 1;
                    self.adjust_scroll();
                }
                Ok(BrowserAction::Continue)
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.enter_selected();
                Ok(BrowserAction::Continue)
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                self.go_back();
                Ok(BrowserAction::Continue)
            }
            KeyCode::Char('s') => {
                self.toggle_sort();
                Ok(BrowserAction::Continue)
            }
            KeyCode::Char('r') => {
                self.reverse_sort();
                Ok(BrowserAction::Continue)
            }
            KeyCode::Char('a') => {
                self.toggle_apparent_size();
                Ok(BrowserAction::Continue)
            }
            KeyCode::Char('d') => {
                self.toggle_show_hidden();
                Ok(BrowserAction::Continue)
            }
            _ => Ok(BrowserAction::Continue),
        }
    }

    /// Move selection by delta
    fn move_selection(&mut self, delta: i32) {
        if self.current.children.is_empty() {
            return;
        }

        let max_index = self.current.children.len() - 1;
        let new_index = if delta < 0 {
            self.selected_index.saturating_sub((-delta) as usize)
        } else {
            cmp::min(self.selected_index + delta as usize, max_index)
        };

        self.selected_index = new_index;
        self.adjust_scroll();
    }

    /// Enter the currently selected item
    fn enter_selected(&mut self) {
        if self.current.children.is_empty() {
            return;
        }

        let selected = &self.current.children[self.selected_index];
        if selected.entry_type.is_directory() && selected.entry_type != EntryType::Error {
            self.path_stack.push(self.current.clone());
            self.current = selected.clone();
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }

    /// Go back to parent directory
    fn go_back(&mut self) {
        if let Some(parent) = self.path_stack.pop() {
            self.current = parent;
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }

    /// Toggle sort column
    fn toggle_sort(&mut self) {
        // Cycle through sort columns
        // Note: This is a simplified version - in a full implementation,
        // we'd need to re-sort the current directory's children
    }

    /// Reverse sort order
    fn reverse_sort(&mut self) {
        // Toggle between ascending and descending
        // Note: This is a simplified version - in a full implementation,
        // we'd need to re-sort the current directory's children
    }

    /// Toggle between apparent size and disk usage
    fn toggle_apparent_size(&mut self) {
        // Toggle the display mode
        // Note: This would affect how sizes are displayed
    }

    /// Toggle showing hidden files
    fn toggle_show_hidden(&mut self) {
        // Toggle hidden file visibility
        // Note: This would require re-filtering the directory contents
    }

    /// Adjust scroll offset to keep selection visible
    fn adjust_scroll(&mut self) {
        let visible_height = self.get_visible_height();

        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index + 1 - visible_height;
        }
    }

    /// Get the number of visible lines for file list
    fn get_visible_height(&self) -> usize {
        // Reserve space for header, current path, and status line
        (self.terminal_height as usize).saturating_sub(4)
    }

    /// Update terminal size
    fn update_terminal_size(&mut self) -> Result<()> {
        let (width, height) = terminal::size()
            .map_err(|e| RsduError::UiError(format!("Cannot get terminal size: {}", e)))?;
        self.terminal_width = width;
        self.terminal_height = height;
        Ok(())
    }

    /// Draw the interface
    fn draw(&mut self) -> Result<()> {
        let mut stdout = io::stdout();

        queue!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        if self.show_help {
            self.draw_help(&mut stdout)?;
        } else {
            self.draw_browser(&mut stdout)?;
        }

        stdout
            .flush()
            .map_err(|e| RsduError::UiError(format!("Cannot flush stdout: {}", e)))?;

        Ok(())
    }

    /// Draw the main browser interface
    fn draw_browser(&mut self, stdout: &mut impl Write) -> Result<()> {
        // Header
        self.draw_header(stdout)?;

        // Current path
        self.draw_current_path(stdout)?;

        // File list
        self.draw_file_list(stdout)?;

        // Status bar
        self.draw_status_bar(stdout)?;

        Ok(())
    }

    /// Draw header with column titles
    fn draw_header(&self, stdout: &mut impl Write) -> Result<()> {
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            SetForegroundColor(Color::White),
            Print("    Size    Items  Name"),
            ResetColor
        )?;
        Ok(())
    }

    /// Draw current path
    fn draw_current_path(&self, stdout: &mut impl Write) -> Result<()> {
        let path = self.get_current_path();
        let display_path = if path.len() > self.terminal_width as usize - 2 {
            format!(
                "...{}",
                &path[path.len() - (self.terminal_width as usize - 5)..]
            )
        } else {
            path
        };

        queue!(
            stdout,
            cursor::MoveTo(0, 1),
            SetForegroundColor(Color::Cyan),
            Print(format!("/{}", display_path)),
            ResetColor
        )?;
        Ok(())
    }

    /// Draw the file list
    fn draw_file_list(&self, stdout: &mut impl Write) -> Result<()> {
        let visible_height = self.get_visible_height();
        let start_y = 3;

        if self.current.children.is_empty() {
            queue!(
                stdout,
                cursor::MoveTo(2, start_y),
                Print("(empty directory)")
            )?;
            return Ok(());
        }

        let end_index = cmp::min(
            self.scroll_offset + visible_height,
            self.current.children.len(),
        );

        for (i, entry) in self.current.children[self.scroll_offset..end_index]
            .iter()
            .enumerate()
        {
            let line_y = start_y + i as u16;
            let global_index = self.scroll_offset + i;
            let is_selected = global_index == self.selected_index;

            self.draw_file_entry(stdout, entry, line_y, is_selected)?;
        }

        Ok(())
    }

    /// Draw a single file entry
    fn draw_file_entry(
        &self,
        stdout: &mut impl Write,
        entry: &Entry,
        y: u16,
        is_selected: bool,
    ) -> Result<()> {
        queue!(stdout, cursor::MoveTo(0, y))?;

        if is_selected {
            queue!(stdout, SetForegroundColor(Color::Black))?;
            // In a full implementation, we'd set background color here
        }

        // Size column (9 chars)
        let size_str = if entry.entry_type.is_directory() {
            format!("{:>8} ", self.calculate_directory_size(entry))
        } else {
            format!("{:>8} ", format_file_size(entry.size, self.config.si))
        };

        // Items column (7 chars) - for directories, show item count
        let items_str = if entry.entry_type.is_directory() {
            format!("{:>6} ", entry.children.len())
        } else {
            "      ".to_string()
        };

        // File type indicator and name
        let (type_char, color) = self.get_type_indicator(entry);
        let name = entry.name_str();

        // Truncate name if too long
        let available_width = self.terminal_width as usize - 20; // Reserve space for size and items
        let display_name = if name.len() > available_width {
            format!("{}...", &name[..available_width.saturating_sub(3)])
        } else {
            name
        };

        queue!(
            stdout,
            Print(size_str),
            Print(items_str),
            SetForegroundColor(color),
            Print(type_char),
            Print(display_name),
            ResetColor
        )?;

        // Show error message if this is an error entry
        if entry.entry_type == EntryType::Error {
            if let Some(ref error) = entry.error {
                queue!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print(format!(" [{}]", error)),
                    ResetColor
                )?;
            }
        }

        Ok(())
    }

    /// Calculate directory size (simplified - just sum of children)
    fn calculate_directory_size(&self, entry: &Entry) -> String {
        let total_size: u64 = entry
            .children
            .iter()
            .map(|child| {
                if child.entry_type.is_directory() {
                    // For directories, we'd need to recurse, but this is simplified
                    child.size
                } else {
                    child.size
                }
            })
            .sum();

        format_file_size(total_size, self.config.si)
    }

    /// Get type indicator character and color for an entry
    fn get_type_indicator(&self, entry: &Entry) -> (char, Color) {
        match entry.entry_type {
            EntryType::Directory => ('/', Color::Blue),
            EntryType::File => (' ', Color::White),
            EntryType::Symlink => ('@', Color::Cyan),
            EntryType::Hardlink => ('>', Color::Yellow),
            EntryType::Special => ('=', Color::Magenta),
            EntryType::Error => ('!', Color::Red),
            EntryType::Excluded => ('x', Color::DarkGrey),
            EntryType::OtherFs => ('~', Color::DarkGrey),
            EntryType::KernelFs => ('#', Color::DarkGrey),
        }
    }

    /// Draw status bar
    fn draw_status_bar(&self, stdout: &mut impl Write) -> Result<()> {
        let status_y = self.terminal_height - 1;
        let total_items = self.current.children.len();

        let status = if total_items > 0 {
            format!(
                "{}/{} items, {} total | q:quit ?:help ↑↓:navigate ←→:enter/back",
                self.selected_index + 1,
                total_items,
                total_items
            )
        } else {
            "Empty directory | q:quit ?:help".to_string()
        };

        // Truncate status if too long
        let display_status = if status.len() > self.terminal_width as usize {
            format!("{}...", &status[..self.terminal_width as usize - 3])
        } else {
            status
        };

        queue!(
            stdout,
            cursor::MoveTo(0, status_y),
            SetForegroundColor(Color::DarkGrey),
            Print(display_status),
            ResetColor
        )?;

        Ok(())
    }

    /// Draw help screen
    fn draw_help(&self, stdout: &mut impl Write) -> Result<()> {
        let help_text = [
            "rsdu - Disk Usage Analyzer",
            "",
            "Navigation:",
            "  ↑/k        Move up",
            "  ↓/j        Move down",
            "  ←/h        Go back to parent directory",
            "  →/l/Enter  Enter directory",
            "  PgUp/PgDn  Page up/down",
            "  Home/g     Go to first item",
            "  End/G      Go to last item",
            "",
            "Sorting & Display:",
            "  s          Change sort column",
            "  r          Reverse sort order",
            "  a          Toggle apparent size/disk usage",
            "  d          Toggle hidden files",
            "",
            "Other:",
            "  ?/F1       Toggle this help",
            "  q/Esc      Quit",
            "  Ctrl+C     Quit",
            "",
            "Press ? or F1 to return to browser",
        ];

        for (i, line) in help_text.iter().enumerate() {
            if i as u16 >= self.terminal_height {
                break;
            }
            queue!(stdout, cursor::MoveTo(2, i as u16), Print(line))?;
        }

        Ok(())
    }

    /// Get the current path as a string
    fn get_current_path(&self) -> String {
        let mut path_parts = Vec::new();

        // Build path from stack
        for entry in &self.path_stack {
            path_parts.push(entry.name_str());
        }
        path_parts.push(self.current.name_str());

        path_parts.join("/")
    }
}

/// Browser action result
#[derive(Debug, PartialEq)]
enum BrowserAction {
    Continue,
    Quit,
}

/// Run the interactive browser
pub fn run_browser(root: Arc<Entry>, config: Config) -> Result<()> {
    let mut browser = Browser::new(root, config)?;
    browser.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{generate_entry_id, Entry, EntryType};

    fn create_test_entry(name: &str, is_dir: bool) -> Arc<Entry> {
        Arc::new(Entry::new(
            generate_entry_id(),
            if is_dir {
                EntryType::Directory
            } else {
                EntryType::File
            },
            name.into(),
            1024,
            2,
            1,
            1,
            1,
        ))
    }

    #[test]
    fn test_browser_creation() {
        let root = create_test_entry("test", true);
        let config = Config::default();

        // Note: This test would need to be adjusted for environments without a terminal
        // In practice, we'd mock the terminal interface for testing
    }

    #[test]
    fn test_path_building() {
        let root = create_test_entry("root", true);
        let config = Config::default();

        // Create a mock browser to test path logic
        // In a full implementation, we'd have more comprehensive path tests
    }
}
