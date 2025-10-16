//! Error handling for rsdu
//!
//! This module defines the error types used throughout the application.

// use std::fmt; // TODO: May be used for custom error formatting
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for rsdu
#[derive(Error, Debug)]
pub enum RsduError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Permission denied accessing '{path}': {source}")]
    PermissionDenied { path: PathBuf, source: io::Error },

    #[error("Path not found: '{path}'")]
    PathNotFound { path: PathBuf },

    #[error("Invalid path: '{path}' - {reason}")]
    InvalidPath { path: PathBuf, reason: String },

    #[error("Scan error in '{path}': {message}")]
    ScanError { path: PathBuf, message: String },

    #[error("Import error: {0}")]
    ImportError(String),

    #[error("Export error: {0}")]
    ExportError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("UI error: {0}")]
    UiError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Thread error: {0}")]
    ThreadError(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("User cancelled operation")]
    UserCancelled,

    #[error("Feature not available: {0}")]
    FeatureNotAvailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, RsduError>;

impl RsduError {
    /// Check if this error is recoverable during scanning
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            RsduError::PermissionDenied { .. }
                | RsduError::PathNotFound { .. }
                | RsduError::ScanError { .. }
        )
    }

    /// Get the path associated with this error, if any
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            RsduError::PermissionDenied { path, .. }
            | RsduError::PathNotFound { path }
            | RsduError::InvalidPath { path, .. }
            | RsduError::ScanError { path, .. } => Some(path),
            _ => None,
        }
    }

    /// Create a permission denied error
    pub fn permission_denied<P: Into<PathBuf>>(path: P, source: io::Error) -> Self {
        Self::PermissionDenied {
            path: path.into(),
            source,
        }
    }

    /// Create a path not found error
    pub fn path_not_found<P: Into<PathBuf>>(path: P) -> Self {
        Self::PathNotFound { path: path.into() }
    }

    /// Create an invalid path error
    pub fn invalid_path<P: Into<PathBuf>>(path: P, reason: impl Into<String>) -> Self {
        Self::InvalidPath {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a scan error
    pub fn scan_error<P: Into<PathBuf>>(path: P, message: impl Into<String>) -> Self {
        Self::ScanError {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// Convert io::Error to RsduError with context
pub fn io_error_with_path<P: Into<PathBuf>>(error: io::Error, path: P) -> RsduError {
    let path = path.into();
    match error.kind() {
        io::ErrorKind::PermissionDenied => RsduError::permission_denied(path, error),
        io::ErrorKind::NotFound => RsduError::path_not_found(path),
        _ => RsduError::Io(error),
    }
}

/// Helper trait for adding path context to Results
pub trait ResultExt<T> {
    fn with_path<P: Into<PathBuf>>(self, path: P) -> Result<T>;
}

impl<T> ResultExt<T> for std::result::Result<T, io::Error> {
    fn with_path<P: Into<PathBuf>>(self, path: P) -> Result<T> {
        self.map_err(|e| io_error_with_path(e, path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let path = PathBuf::from("/test/path");
        let error = RsduError::path_not_found(&path);
        assert_eq!(error.path(), Some(&path));
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_permission_error() {
        let path = PathBuf::from("/restricted");
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let error = RsduError::permission_denied(&path, io_err);

        match error {
            RsduError::PermissionDenied { path: p, .. } => assert_eq!(p, path),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_io_error_conversion() {
        let path = PathBuf::from("/test");
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = io_error_with_path(io_err, &path);

        match error {
            RsduError::PathNotFound { path: p } => assert_eq!(p, path),
            _ => panic!("Wrong error type"),
        }
    }
}
