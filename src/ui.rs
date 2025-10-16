//! Terminal UI module
//!
//! This module handles terminal initialization, cleanup, and basic UI operations
//! for the interactive interface.

use crate::error::{Result, RsduError};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

/// UI state and terminal handle
pub struct UI {
    /// Whether the terminal has been initialized
    initialized: bool,
}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Initialize the terminal for full-screen operation
    pub fn init(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Enable raw mode and alternate screen
        terminal::enable_raw_mode()
            .map_err(|e| RsduError::UiError(format!("Failed to enable raw mode: {}", e)))?;

        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)
            .map_err(|e| RsduError::UiError(format!("Failed to setup terminal: {}", e)))?;

        self.initialized = true;
        Ok(())
    }

    /// Cleanup and restore terminal
    pub fn cleanup(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show)
            .map_err(|e| RsduError::UiError(format!("Failed to restore terminal: {}", e)))?;

        terminal::disable_raw_mode()
            .map_err(|e| RsduError::UiError(format!("Failed to disable raw mode: {}", e)))?;

        self.initialized = false;
        Ok(())
    }

    /// Clear the screen
    pub fn clear(&self) -> Result<()> {
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .map_err(|e| RsduError::UiError(format!("Failed to clear screen: {}", e)))?;
        Ok(())
    }

    /// Get terminal size
    pub fn size(&self) -> Result<(u16, u16)> {
        terminal::size()
            .map_err(|e| RsduError::UiError(format!("Failed to get terminal size: {}", e)))
    }

    /// Wait for a key press and return it
    pub fn wait_for_key(&self) -> Result<KeyCode> {
        loop {
            match event::read()
                .map_err(|e| RsduError::UiError(format!("Failed to read event: {}", e)))?
            {
                Event::Key(key_event) => {
                    return Ok(key_event.code);
                }
                _ => continue,
            }
        }
    }

    /// Check if a key is available without blocking
    pub fn poll_key(&self) -> Result<Option<KeyCode>> {
        if event::poll(std::time::Duration::from_millis(0))
            .map_err(|e| RsduError::UiError(format!("Failed to poll events: {}", e)))?
        {
            match event::read()
                .map_err(|e| RsduError::UiError(format!("Failed to read event: {}", e)))?
            {
                Event::Key(key_event) => Ok(Some(key_event.code)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Move cursor to position
    pub fn move_cursor(&self, x: u16, y: u16) -> Result<()> {
        execute!(io::stdout(), cursor::MoveTo(x, y))
            .map_err(|e| RsduError::UiError(format!("Failed to move cursor: {}", e)))?;
        Ok(())
    }

    /// Print text at current cursor position
    pub fn print(&self, text: &str) -> Result<()> {
        print!("{}", text);
        io::stdout()
            .flush()
            .map_err(|e| RsduError::UiError(format!("Failed to flush output: {}", e)))?;
        Ok(())
    }

    /// Print text at specific position
    pub fn print_at(&self, x: u16, y: u16, text: &str) -> Result<()> {
        self.move_cursor(x, y)?;
        self.print(text)
    }
}

impl Drop for UI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Initialize the UI system
pub fn initialize() -> Result<UI> {
    let mut ui = UI::new();
    ui.init()?;
    Ok(ui)
}

/// Display an out-of-memory error and exit
pub fn show_oom_error() -> ! {
    eprintln!("rsdu: out of memory");
    std::process::exit(1);
}

/// Display an error message and exit
pub fn fatal_error(message: &str) -> ! {
    eprintln!("rsdu: {}", message);
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_creation() {
        let ui = UI::new();
        assert!(!ui.initialized);
    }

    // Note: Most UI tests would require a TTY and are difficult to test in CI
    // Integration tests should cover the full UI functionality
}
