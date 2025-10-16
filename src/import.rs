//! Data import module
//!
//! This module handles importing previously exported data from JSON and binary formats.

use crate::error::{Result, RsduError};
use crate::model::{Entry, SerializableEntry};
// use crate::model::{generate_entry_id, EntryType}; // TODO: Will be used for entry creation
use serde_json;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::sync::Arc;

/// Import data from stdin
pub fn import_from_stdin() -> Result<Arc<Entry>> {
    let stdin = io::stdin();
    let reader = stdin.lock();
    import_from_reader(reader)
}

/// Import data from a file
pub fn import_from_file(path: &Path) -> Result<Arc<Entry>> {
    let file = File::open(path)
        .map_err(|e| RsduError::ImportError(format!("Failed to open import file: {}", e)))?;

    let reader = BufReader::new(file);

    // For now, assume JSON format
    import_from_reader(reader)
}

/// Import data from any reader
fn import_from_reader<R: Read>(mut reader: R) -> Result<Arc<Entry>> {
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(|e| RsduError::ImportError(format!("Failed to read import data: {}", e)))?;

    // Try to parse as JSON
    if let Ok(serializable_entry) = serde_json::from_str::<SerializableEntry>(&content) {
        return Ok(Entry::from_serializable(serializable_entry));
    }

    // If JSON parsing fails, try binary format
    // TODO: Implement binary format parsing

    Err(RsduError::ImportError(
        "Unknown or invalid import format".to_string(),
    ))
}

/// Import from JSON string
pub fn import_from_json(json: &str) -> Result<Arc<Entry>> {
    let serializable_entry: SerializableEntry = serde_json::from_str(json)
        .map_err(|e| RsduError::ImportError(format!("Invalid JSON format: {}", e)))?;

    Ok(Entry::from_serializable(serializable_entry))
}

/// Import from binary data
pub fn import_from_binary(_data: &[u8]) -> Result<Arc<Entry>> {
    // TODO: Implement binary format parsing
    // This would involve parsing the binary export format from ncdu

    Err(RsduError::ImportError(
        "Binary import not yet implemented".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryType;

    #[test]
    fn test_json_import() {
        let json = r#"{
            "id": 1,
            "entry_type": "File",
            "name": "test.txt",
            "size": 1024,
            "blocks": 2,
            "device": 1,
            "inode": 12345,
            "nlink": 1,
            "extended": null,
            "error": null,
            "children": []
        }"#;

        let result = import_from_json(json);
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.entry_type, EntryType::File);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_invalid_json() {
        let invalid_json = "{ invalid json }";
        let result = import_from_json(invalid_json);
        assert!(result.is_err());
    }
}
