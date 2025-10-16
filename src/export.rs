//! Data export module
//!
//! This module handles exporting scanned directory data to JSON and binary formats.

use crate::error::{Result, RsduError};
use crate::model::Entry;
use serde_json;
use std::fs::File;
use std::io::{self, BufWriter, Write};
// use std::path::Path; // TODO: Will be used for path operations
// use std::sync::Arc; // TODO: Will be used for Arc<Entry>

/// Export handler for managing output
pub struct ExportHandler {
    writer: Box<dyn Write + Send>,
    format: ExportFormat,
    compress: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Binary,
}

impl ExportHandler {
    /// Create a new export handler for JSON format
    pub fn json<W: Write + Send + 'static>(writer: W, compress: bool) -> Self {
        Self {
            writer: Box::new(writer),
            format: ExportFormat::Json,
            compress,
        }
    }

    /// Create a new export handler for binary format
    pub fn binary<W: Write + Send + 'static>(writer: W, compress: bool) -> Self {
        Self {
            writer: Box::new(writer),
            format: ExportFormat::Binary,
            compress,
        }
    }

    /// Export an entry tree
    pub fn export(&mut self, entry: &Entry) -> Result<()> {
        match self.format {
            ExportFormat::Json => self.export_json(entry),
            ExportFormat::Binary => self.export_binary(entry),
        }
    }

    /// Export to JSON format
    fn export_json(&mut self, entry: &Entry) -> Result<()> {
        let serializable = entry.to_serializable();
        let json = serde_json::to_string_pretty(&serializable)
            .map_err(|e| RsduError::ExportError(format!("JSON serialization failed: {}", e)))?;

        if self.compress {
            // TODO: Implement compression
            self.writer
                .write_all(json.as_bytes())
                .map_err(|e| RsduError::ExportError(format!("Write failed: {}", e)))?;
        } else {
            self.writer
                .write_all(json.as_bytes())
                .map_err(|e| RsduError::ExportError(format!("Write failed: {}", e)))?;
        }

        self.writer
            .flush()
            .map_err(|e| RsduError::ExportError(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Export to binary format
    fn export_binary(&mut self, _entry: &Entry) -> Result<()> {
        // TODO: Implement binary export format compatible with ncdu
        Err(RsduError::ExportError(
            "Binary export not yet implemented".to_string(),
        ))
    }
}

/// Setup JSON export to a file
pub fn setup_json_export(filename: &str) -> Result<ExportHandler> {
    let writer: Box<dyn Write + Send> = if filename == "-" {
        Box::new(io::stdout())
    } else {
        let file = File::create(filename).map_err(|e| {
            RsduError::ExportError(format!(
                "Failed to create export file '{}': {}",
                filename, e
            ))
        })?;
        Box::new(BufWriter::new(file))
    };

    Ok(ExportHandler::json(writer, false))
}

/// Setup binary export to a file
pub fn setup_binary_export(filename: &str) -> Result<ExportHandler> {
    let writer: Box<dyn Write + Send> = if filename == "-" {
        Box::new(io::stdout())
    } else {
        let file = File::create(filename).map_err(|e| {
            RsduError::ExportError(format!(
                "Failed to create export file '{}': {}",
                filename, e
            ))
        })?;
        Box::new(BufWriter::new(file))
    };

    Ok(ExportHandler::binary(writer, false))
}

/// Export entry tree to JSON string
pub fn export_to_json_string(entry: &Entry) -> Result<String> {
    let serializable = entry.to_serializable();
    serde_json::to_string_pretty(&serializable)
        .map_err(|e| RsduError::ExportError(format!("JSON serialization failed: {}", e)))
}

/// Export entry tree to compact JSON string
pub fn export_to_json_compact(entry: &Entry) -> Result<String> {
    let serializable = entry.to_serializable();
    serde_json::to_string(&serializable)
        .map_err(|e| RsduError::ExportError(format!("JSON serialization failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{generate_entry_id, EntryType};
    use std::ffi::OsString;

    #[test]
    fn test_json_export() {
        let entry = Entry::new(
            generate_entry_id(),
            EntryType::File,
            OsString::from("test.txt"),
            1024,
            2,
            1,
            12345,
            1,
        );

        let result = export_to_json_string(&entry);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_json_compact_export() {
        let entry = Entry::new(
            generate_entry_id(),
            EntryType::Directory,
            OsString::from("testdir"),
            0,
            0,
            1,
            54321,
            2,
        );

        let result = export_to_json_compact(&entry);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("testdir"));
        // Compact format should not have pretty formatting
        assert!(!json.contains("  "));
    }

    #[test]
    fn test_export_handler_creation() {
        let buffer = Vec::new();
        let handler = ExportHandler::json(buffer, false);
        assert!(matches!(handler.format, ExportFormat::Json));
    }
}
