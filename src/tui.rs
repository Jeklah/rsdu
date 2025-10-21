//! Modern TUI implementation using ratatui
//!
//! This module provides a proper terminal user interface with:
//! - Scanning progress screen with progress bar
//! - File browser with ncdu-like appearance
//! - Proper event handling and state management
//! - Clean transitions between modes

use crate::config::Config;
use crate::error::{Result, RsduError};
use crate::model::{Entry, EntryType, ScanStats};
use crate::utils::format_file_size;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{block::Title, Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// TUI application state
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
    mode: AppMode,
}

/// Application modes
#[derive(Debug)]
pub enum AppMode {
    Scanning {
        progress: Arc<ScanProgress>,
        receiver: Option<Receiver<ScanMessage>>,
    },
    Browsing {
        root: Arc<Entry>,
        current_dir: Arc<Entry>,
        path_stack: Vec<Arc<Entry>>,
        list_state: ListState,
        show_help: bool,
    },
    Quit,
}

/// Scanning progress information
#[derive(Debug)]
pub struct ScanProgress {
    pub current_path: Mutex<String>,
    pub total_entries: AtomicUsize,
    pub directories: AtomicUsize,
    pub files: AtomicUsize,
    pub errors: AtomicUsize,
    pub total_size: AtomicUsize,
    pub is_complete: AtomicBool,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            current_path: Mutex::new(String::new()),
            total_entries: AtomicUsize::new(0),
            directories: AtomicUsize::new(0),
            files: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            total_size: AtomicUsize::new(0),
            is_complete: AtomicBool::new(false),
        }
    }
}

/// Simple stats for progress messages
#[derive(Debug, Clone)]
pub struct ProgressStats {
    pub total_entries: u64,
    pub directories: u64,
    pub files: u64,
    pub errors: u64,
    pub total_size: u64,
}

impl ProgressStats {
    pub fn from_scan_stats(stats: &ScanStats) -> Self {
        Self {
            total_entries: stats.get_total_entries(),
            directories: stats.get_directories(),
            files: stats.get_files(),
            errors: stats.get_errors(),
            total_size: stats.get_total_size(),
        }
    }
}

/// Messages sent during scanning
#[derive(Debug, Clone)]
pub enum ScanMessage {
    Progress {
        current_path: String,
        stats: ProgressStats,
    },
    Complete {
        root: Arc<Entry>,
    },
    Error {
        message: String,
    },
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(config: Config) -> Result<Self> {
        // Setup terminal
        enable_raw_mode()
            .map_err(|e| RsduError::UiError(format!("Failed to enable raw mode: {}", e)))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| RsduError::UiError(format!("Failed to setup terminal: {}", e)))?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| RsduError::UiError(format!("Failed to create terminal: {}", e)))?;

        Ok(Self {
            terminal,
            config,
            mode: AppMode::Quit, // Will be set when starting scan
        })
    }

    /// Start scanning with progress display
    pub fn start_scan(&mut self, scan_path: String) -> Result<Sender<ScanMessage>> {
        let progress = Arc::new(ScanProgress::default());
        let (sender, receiver) = mpsc::channel();

        self.mode = AppMode::Scanning {
            progress: progress.clone(),
            receiver: Some(receiver),
        };

        // Update initial path
        if let Ok(mut current_path) = progress.current_path.lock() {
            *current_path = scan_path;
        }

        Ok(sender)
    }

    /// Run the main application loop
    pub fn run(&mut self) -> Result<()> {
        let mut last_tick = Instant::now();
        let mut last_ui_update = Instant::now();
        let tick_rate = Duration::from_millis(50); // Faster tick rate for scanning updates
        let ui_update_rate = Duration::from_millis(100); // UI refresh rate

        loop {
            // Handle updates first
            if last_tick.elapsed() >= tick_rate {
                self.update()?;
                last_tick = Instant::now();
            }

            // Draw the UI at a controlled rate to avoid flickering
            let should_draw = match &self.mode {
                AppMode::Scanning { .. } => last_ui_update.elapsed() >= ui_update_rate,
                _ => true, // Always draw for browsing mode
            };

            if should_draw {
                let should_quit = {
                    let mode_ref = &self.mode;
                    self.terminal
                        .draw(|f| draw_ui_for_mode(f, mode_ref, &self.config))
                        .map_err(|e| RsduError::UiError(format!("Failed to draw: {}", e)))?;
                    matches!(self.mode, AppMode::Quit)
                };

                if should_quit {
                    break;
                }
                last_ui_update = Instant::now();
            }

            // Handle input
            let timeout = Duration::from_millis(10); // Short timeout for responsiveness
            if event::poll(timeout)
                .map_err(|e| RsduError::UiError(format!("Event poll error: {}", e)))?
            {
                if let Event::Key(key) = event::read()
                    .map_err(|e| RsduError::UiError(format!("Event read error: {}", e)))?
                {
                    if key.kind == KeyEventKind::Press {
                        if self.handle_key_event(key.code)? {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Update application state
    fn update(&mut self) -> Result<()> {
        match &mut self.mode {
            AppMode::Scanning { receiver, progress } => {
                if let Some(rx) = receiver {
                    // Process multiple messages per update but limit to avoid blocking UI
                    let mut processed = 0;
                    while processed < 10 {
                        match rx.try_recv() {
                            Ok(msg) => {
                                processed += 1;
                                match msg {
                                    ScanMessage::Progress {
                                        current_path,
                                        stats,
                                    } => {
                                        if let Ok(mut path) = progress.current_path.lock() {
                                            *path = current_path;
                                        }
                                        progress
                                            .total_entries
                                            .store(stats.total_entries as usize, Ordering::Relaxed);
                                        progress
                                            .directories
                                            .store(stats.directories as usize, Ordering::Relaxed);
                                        progress
                                            .files
                                            .store(stats.files as usize, Ordering::Relaxed);
                                        progress
                                            .errors
                                            .store(stats.errors as usize, Ordering::Relaxed);
                                        progress
                                            .total_size
                                            .store(stats.total_size as usize, Ordering::Relaxed);
                                    }
                                    ScanMessage::Complete { root } => {
                                        progress.is_complete.store(true, Ordering::Relaxed);
                                        self.start_browsing(root)?;
                                        return Ok(());
                                    }
                                    ScanMessage::Error { message } => {
                                        return Err(RsduError::ScanError {
                                            path: std::path::PathBuf::from("unknown"),
                                            message,
                                        });
                                    }
                                }
                            }
                            Err(_) => break, // No more messages available
                        }
                    }
                }
            }
            AppMode::Browsing { .. } => {
                // Nothing to update in browsing mode
            }
            AppMode::Quit => {}
        }
        Ok(())
    }

    /// Switch to browsing mode
    fn start_browsing(&mut self, root: Arc<Entry>) -> Result<()> {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        self.mode = AppMode::Browsing {
            current_dir: root.clone(),
            root,
            path_stack: Vec::new(),
            list_state,
            show_help: false,
        };
        Ok(())
    }

    /// Handle keyboard events
    fn handle_key_event(&mut self, key: KeyCode) -> Result<bool> {
        match &mut self.mode {
            AppMode::Scanning { .. } => {
                match key {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('c') => {
                        return Ok(true); // Quit
                    }
                    _ => {}
                }
            }
            AppMode::Browsing {
                current_dir,
                path_stack,
                list_state,
                show_help,
                ..
            } => {
                match key {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if *show_help {
                            *show_help = false;
                        } else {
                            return Ok(true); // Quit
                        }
                    }
                    KeyCode::Char('?') | KeyCode::F(1) => {
                        *show_help = !*show_help;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !*show_help {
                            self.move_selection(-1);
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !*show_help {
                            self.move_selection(1);
                        }
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        if !*show_help {
                            list_state.select(Some(0));
                        }
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        if !*show_help && !current_dir.children.is_empty() {
                            list_state.select(Some(current_dir.children.len() - 1));
                        }
                    }
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        if !*show_help {
                            self.enter_selected()?;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                        if !*show_help && !path_stack.is_empty() {
                            let parent = path_stack.pop().unwrap();
                            *current_dir = parent;
                            list_state.select(Some(0));
                        }
                    }
                    _ => {}
                }
            }
            AppMode::Quit => {}
        }
        Ok(false)
    }

    /// Move selection up or down
    fn move_selection(&mut self, delta: i32) {
        if let AppMode::Browsing {
            current_dir,
            list_state,
            ..
        } = &mut self.mode
        {
            if current_dir.children.is_empty() {
                return;
            }

            let current = list_state.selected().unwrap_or(0);
            let max_index = current_dir.children.len() - 1;

            let new_index = if delta < 0 {
                current.saturating_sub((-delta) as usize)
            } else {
                (current + delta as usize).min(max_index)
            };

            list_state.select(Some(new_index));
        }
    }

    /// Enter the currently selected directory
    fn enter_selected(&mut self) -> Result<()> {
        if let AppMode::Browsing {
            current_dir,
            path_stack,
            list_state,
            ..
        } = &mut self.mode
        {
            if let Some(selected_index) = list_state.selected() {
                if selected_index < current_dir.children.len() {
                    let selected = &current_dir.children[selected_index];
                    if selected.entry_type.is_directory() && selected.entry_type != EntryType::Error
                    {
                        path_stack.push(current_dir.clone());
                        *current_dir = selected.clone();
                        list_state.select(Some(0));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Draw UI for the given mode (standalone function to avoid borrowing issues)
fn draw_ui_for_mode(f: &mut Frame, mode: &AppMode, config: &Config) {
    match mode {
        AppMode::Scanning { progress, .. } => {
            draw_scanning_ui_standalone(f, progress, config);
        }
        AppMode::Browsing {
            show_help: true, ..
        } => {
            draw_help_ui_standalone(f);
        }
        AppMode::Browsing {
            root: _,
            current_dir,
            path_stack,
            list_state,
            ..
        } => {
            draw_browsing_ui_standalone(f, current_dir, path_stack, list_state, config);
        }
        AppMode::Quit => {}
    }
}

/// Enhanced scanning UI function with ncdu-like appearance
fn draw_scanning_ui_standalone(f: &mut Frame, progress: &Arc<ScanProgress>, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Current file being scanned (larger)
            Constraint::Length(4), // Progress info
            Constraint::Min(6),    // Statistics (larger)
            Constraint::Length(2), // Instructions
        ])
        .split(f.size());

    // Title - ncdu style
    let title = Paragraph::new("ncdu - Disk Usage Analyzer")
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Current file being scanned - prominent display like ncdu
    let current_path = progress.current_path.lock().unwrap().clone();
    let truncated_path = if current_path.len() > (chunks[1].width as usize).saturating_sub(6) {
        let max_len = (chunks[1].width as usize).saturating_sub(9); // Leave room for "..."
        if current_path.len() > max_len {
            format!("...{}", &current_path[current_path.len() - max_len..])
        } else {
            current_path.clone()
        }
    } else {
        current_path.clone()
    };

    let current_file_widget = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Scanning: "),
            Span::styled(
                truncated_path,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ]))
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Left);
    f.render_widget(current_file_widget, chunks[1]);

    // Progress information
    let total_entries = progress.total_entries.load(Ordering::Relaxed);
    let directories = progress.directories.load(Ordering::Relaxed);
    let files = progress.files.load(Ordering::Relaxed);

    let progress_text = vec![
        Line::from(vec![
            Span::raw("Total items: "),
            Span::styled(
                total_entries.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ("),
            Span::styled(directories.to_string(), Style::default().fg(Color::Blue)),
            Span::raw(" dirs, "),
            Span::styled(files.to_string(), Style::default().fg(Color::Green)),
            Span::raw(" files)"),
        ]),
        Line::from(""),
    ];

    let progress_info = Paragraph::new(Text::from(progress_text))
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .alignment(Alignment::Left);
    f.render_widget(progress_info, chunks[2]);

    // Statistics - more detailed like ncdu
    let total_size = progress.total_size.load(Ordering::Relaxed) as u64;
    let errors = progress.errors.load(Ordering::Relaxed);

    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Total size: "),
            Span::styled(
                format_file_size(total_size, config.si),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        if errors > 0 {
            Line::from(vec![
                Span::raw("  Errors: "),
                Span::styled(
                    errors.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from("")
        },
        Line::from(""),
    ];

    let stats_widget = Paragraph::new(Text::from(stats_text))
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .alignment(Alignment::Left);
    f.render_widget(stats_widget, chunks[3]);

    // Instructions
    let instructions = Paragraph::new("Press q to quit, or wait for scan to complete...")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[4]);
}

/// Standalone help UI function
fn draw_help_ui_standalone(f: &mut Frame) {
    let help_text = vec![
        Line::from(Span::styled(
            "rsdu - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/k        Move up"),
        Line::from("  ↓/j        Move down"),
        Line::from("  ←/h        Go back to parent directory"),
        Line::from("  →/l/Enter  Enter directory"),
        Line::from("  Home/g     Go to first item"),
        Line::from("  End/G      Go to last item"),
        Line::from(""),
        Line::from("Other:"),
        Line::from("  ?/F1       Toggle this help"),
        Line::from("  q/Esc      Quit"),
        Line::from(""),
        Line::from("Press ? or Esc to return to browser"),
    ];

    // Center the help dialog
    let area = centered_rect(60, 70, f.size());
    f.render_widget(Clear, area);

    let help_widget = Paragraph::new(Text::from(help_text))
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });
    f.render_widget(help_widget, area);
}

/// Standalone browsing UI function
fn draw_browsing_ui_standalone(
    f: &mut Frame,
    current_dir: &Arc<Entry>,
    path_stack: &[Arc<Entry>],
    list_state: &ListState,
    config: &Config,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // File list
            Constraint::Length(3), // Status line
        ])
        .split(f.size());

    // Header with current path and total size
    let current_path = build_current_path(path_stack, current_dir);
    let total_size = calculate_total_size(current_dir);

    let header_text = vec![
        Line::from(vec![
            Span::raw("Path: "),
            Span::styled(&current_path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Total: "),
            Span::styled(
                format_file_size(total_size, config.si),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" ("),
            Span::styled(
                format!("{} items", current_dir.children.len()),
                Style::default().fg(Color::Green),
            ),
            Span::raw(")"),
        ]),
    ];

    let header = Paragraph::new(Text::from(header_text)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Title::from("rsdu - Disk Usage Analyzer").alignment(Alignment::Center)),
    );
    f.render_widget(header, chunks[0]);

    // File list
    if current_dir.children.is_empty() {
        let empty_msg = Paragraph::new("(empty directory)")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(empty_msg, chunks[1]);
    } else {
        let items = create_file_list_items(current_dir, chunks[1].width as usize, config.si);
        let file_list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        f.render_stateful_widget(file_list, chunks[1], &mut list_state.clone());
    }

    // Status line
    let selected_index = list_state.selected().unwrap_or(0);
    let status_text = if current_dir.children.is_empty() {
        "Empty directory | q:quit ?:help".to_string()
    } else {
        format!(
            "{}/{} | q:quit ?:help ↑↓:navigate ←→:dir Enter:enter h:up",
            selected_index + 1,
            current_dir.children.len()
        )
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(status, chunks[2]);
}

/// Create file list items with proper formatting
fn create_file_list_items(
    current_dir: &Arc<Entry>,
    available_width: usize,
    use_si: bool,
) -> Vec<ListItem> {
    let mut items = Vec::new();

    // Calculate column widths - set to match the 10-character size padding
    let size_width = 10;
    let bar_width = 15;
    let spacing = 2;
    let name_width = available_width.saturating_sub(size_width + bar_width + spacing + 4); // 4 for borders

    // Calculate total size for percentage bars
    let total_size = calculate_total_size(current_dir);

    for entry in &current_dir.children {
        let entry_size = if entry.entry_type.is_directory() {
            calculate_directory_size(entry)
        } else {
            entry.size
        };

        // Format size (now properly padded by format_file_size function)
        let size_str = format_file_size(entry_size, use_si);

        // Create percentage bar
        let percentage = if total_size > 0 {
            (entry_size as f64 / total_size as f64 * 100.0) as u8
        } else {
            0
        };
        let bar = create_percentage_bar(percentage, bar_width.saturating_sub(2));

        // Get file type info
        let (type_char, color) = get_file_type_info(entry);

        // Format name with type indicator
        let name_with_type = format!("{}{}", type_char, entry.name_str());
        let truncated_name = if name_with_type.width() > name_width {
            let mut truncated = String::new();
            let mut current_width = 0;
            for ch in name_with_type.chars() {
                let char_width = ch.width().unwrap_or(0);
                if current_width + char_width + 3 > name_width {
                    // 3 for "..."
                    truncated.push_str("...");
                    break;
                }
                truncated.push(ch);
                current_width += char_width;
            }
            truncated
        } else {
            name_with_type
        };

        // Create the line
        let line = Line::from(vec![
            Span::styled(size_str, Style::default().fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(format!("[{}]", bar), Style::default().fg(Color::Blue)),
            Span::raw(" "),
            Span::styled(truncated_name, Style::default().fg(color)),
        ]);

        items.push(ListItem::new(line));
    }

    items
}

/// Create a percentage bar string
fn create_percentage_bar(percentage: u8, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let filled = (percentage as usize * width / 100).min(width);
    let mut bar = String::new();

    for i in 0..width {
        if i < filled {
            bar.push('█');
        } else {
            bar.push(' ');
        }
    }

    bar
}

/// Get file type character and color
fn get_file_type_info(entry: &Entry) -> (char, Color) {
    match entry.entry_type {
        EntryType::Directory => ('/', Color::Blue),
        EntryType::File => (' ', Color::White),
        EntryType::Symlink => ('@', Color::Cyan),
        EntryType::Hardlink => ('>', Color::Yellow),
        EntryType::Special => ('=', Color::Magenta),
        EntryType::Error => ('!', Color::Red),
        EntryType::Excluded => ('x', Color::DarkGray),
        EntryType::OtherFs => ('~', Color::DarkGray),
        EntryType::KernelFs => ('#', Color::DarkGray),
    }
}

/// Build current path string
fn build_current_path(path_stack: &[Arc<Entry>], current_dir: &Arc<Entry>) -> String {
    let mut path_parts = Vec::new();
    for entry in path_stack {
        path_parts.push(entry.name_str());
    }
    path_parts.push(current_dir.name_str());
    format!("/{}", path_parts.join("/"))
}

/// Calculate total size of current directory
fn calculate_total_size(dir: &Arc<Entry>) -> u64 {
    dir.children
        .iter()
        .map(|entry| {
            if entry.entry_type.is_directory() {
                calculate_directory_size(entry)
            } else {
                entry.size
            }
        })
        .sum()
}

/// Calculate directory size (simplified)
fn calculate_directory_size(entry: &Entry) -> u64 {
    entry.size
        + entry
            .children
            .iter()
            .map(|child| {
                if child.entry_type.is_directory() {
                    calculate_directory_size(child)
                } else {
                    child.size
                }
            })
            .sum::<u64>()
}

/// Create centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Cleanup terminal
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}
