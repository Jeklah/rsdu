//! rsdu - A disk usage analyzer with an ncurses interface
//!
//! This is a Rust implementation of ncdu (NCurses Disk Usage), providing
//! fast directory scanning and an interactive terminal interface for
//! exploring disk usage.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod browser;
mod cli;
mod config;
mod error;
mod export;
mod import;
mod model;
mod scanner;
mod tui;

mod utils;

use cli::Args;
use config::Config;
use scanner::scan_directory_with_progress;
use tui::TuiApp;

/// Main entry point for rsdu
fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize configuration from args and config files
    let mut config = Config::from_args(&args)?;

    // Handle version and help (clap handles these automatically)

    // If we're importing from a file, handle that
    if let Some(import_file) = &args.import_file {
        return handle_import(import_file, &config);
    }

    // If we're exporting, set up export and continue with scan
    let _export_handler = if let Some(export_file) = &args.export_json {
        Some(export::setup_json_export(export_file)?)
    } else if let Some(export_file) = &args.export_binary {
        Some(export::setup_binary_export(export_file)?)
    } else {
        None
    };

    // Determine the directory to scan
    let scan_path = args
        .directory
        .as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| std::path::Path::new("."));

    // Canonicalize the path
    let scan_path = scan_path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Cannot access directory '{}': {}", scan_path.display(), e))?;

    // Update config based on scan mode
    if args.export_json.is_some() || args.export_binary.is_some() {
        if config.scan_ui.is_none() {
            config.scan_ui = Some(if atty::is(atty::Stream::Stdout) {
                config::ScanUi::Line
            } else {
                config::ScanUi::None
            });
        }
    } else {
        // For the stub implementation, use Line mode to avoid terminal initialization issues
        config.scan_ui = Some(config::ScanUi::Line);
    }

    // Start the main application flow
    run_application(scan_path, config)
}

/// Handle importing data from a file
fn handle_import(import_file: &str, config: &Config) -> Result<()> {
    let root = if import_file == "-" {
        import::import_from_stdin()?
    } else {
        let path = PathBuf::from(import_file);
        import::import_from_file(&path)?
    };

    // Start the browser with imported data
    browser::run_browser(root, config.clone()).map_err(|e| anyhow::anyhow!("{}", e))
}

/// Main application flow: scan and then browse (or export)
fn run_application(scan_path: PathBuf, config: Config) -> Result<()> {
    // Check if we should use TUI mode
    let use_tui = config.scan_ui != Some(config::ScanUi::None)
        && config.export_json.is_none()
        && config.export_binary.is_none()
        && atty::is(atty::Stream::Stdout);

    if use_tui {
        // Use the new TUI system
        let mut app = TuiApp::new(config.clone())?;
        let sender = app.start_scan(scan_path.display().to_string())?;

        // Start scanning in background thread
        let scan_path_clone = scan_path.clone();
        let config_clone = config.clone();
        std::thread::spawn(move || {
            if let Err(e) =
                scan_directory_with_progress(&scan_path_clone, &config_clone, Some(sender.clone()))
            {
                let _ = sender.send(tui::ScanMessage::Error {
                    message: format!("Scan failed: {}", e),
                });
            }
        });

        // Run the TUI
        app.run()?;
    } else {
        // Use the old non-TUI mode
        let root = scanner::scan_directory(&scan_path, &config)?;

        // If we're just exporting, we're done
        if config.export_json.is_some() || config.export_binary.is_some() {
            return Ok(());
        }

        // Start the old browser (fallback)
        browser::run_browser(root, config).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_functionality() {
        // Basic smoke test
        assert!(true);
    }
}
