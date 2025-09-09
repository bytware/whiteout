use std::fmt;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Whiteout operations
#[derive(Error, Debug)]
pub enum WhiteoutError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Git operation failed: {0}")]
    Git(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Operation not supported: {0}")]
    NotSupported(String),
}

/// Parse-related errors
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid decoration syntax at line {line}: {message}")]
    InvalidSyntax { line: usize, message: String },
    
    #[error("Regex compilation failed: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("Invalid line range: {start}-{end}")]
    InvalidRange { start: usize, end: usize },
    
    #[error("Unclosed block starting at line {line}")]
    UnclosedBlock { line: usize },
    
    #[error("Mismatched decoration markers")]
    MismatchedMarkers,
    
    #[error("Escaped decoration at line {line}")]
    EscapedDecoration { line: usize },
}

/// Storage-related errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Failed to access storage at {path:?}: {message}")]
    AccessError { path: PathBuf, message: String },
    
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("Decryption error: {0}")]
    Decryption(String),
    
    #[error("Storage corrupted: {0}")]
    Corrupted(String),
    
    #[error("Key not found: {key} in file {file}")]
    KeyNotFound { key: String, file: String },
    
    #[error("Storage locked by another process")]
    Locked,
    
    #[error("Invalid storage format")]
    InvalidFormat,
}

/// Security-related errors
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Path traversal detected: {path:?}")]
    PathTraversal { path: PathBuf },
    
    #[error("Insufficient permissions for {path:?}")]
    InsufficientPermissions { path: PathBuf },
    
    #[error("Invalid salt format")]
    InvalidSalt,
    
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    #[error("TOCTOU race condition detected")]
    RaceCondition,
    
    #[error("Suspicious pattern detected: {pattern}")]
    SuspiciousPattern { pattern: String },
    
    #[error("Command injection attempt: {command}")]
    CommandInjection { command: String },
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found at {path:?}")]
    NotFound { path: PathBuf },
    
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },
    
    #[error("Configuration version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}

/// Result type alias for Whiteout operations
pub type WhiteoutResult<T> = Result<T, WhiteoutError>;

/// Helper trait for adding context to errors
pub trait ErrorContext<T> {
    fn context<C>(self, context: C) -> WhiteoutResult<T>
    where
        C: fmt::Display + Send + Sync + 'static;
        
    fn with_context<C, F>(self, f: F) -> WhiteoutResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T> ErrorContext<T> for WhiteoutResult<T> {
    fn context<C>(self, context: C) -> WhiteoutResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| {
            WhiteoutError::InvalidInput(format!("{}: {}", context, e))
        })
    }
    
    fn with_context<C, F>(self, f: F) -> WhiteoutResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| {
            WhiteoutError::InvalidInput(format!("{}: {}", f(), e))
        })
    }
}

impl<T> ErrorContext<T> for Result<T, io::Error> {
    fn context<C>(self, context: C) -> WhiteoutResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| {
            WhiteoutError::Io(io::Error::new(e.kind(), format!("{}: {}", context, e)))
        })
    }
    
    fn with_context<C, F>(self, f: F) -> WhiteoutResult<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| {
            WhiteoutError::Io(io::Error::new(e.kind(), format!("{}: {}", f(), e)))
        })
    }
}

/// Error display helper for CLI
pub fn display_error(error: &WhiteoutError) {
    use colored::Colorize;
    use std::error::Error;
    
    eprintln!("\n{} {}", "✗".bright_red().bold(), "Operation failed".bright_red().bold());
    eprintln!("  {} {}", "├".bright_black(), error);
    
    // Display error chain
    let mut source = error.source();
    while let Some(err) = source {
        eprintln!("  {} Caused by: {}", "├".bright_black(), err);
        source = err.source();
    }
    
    // Add helpful hints based on error type
    match error {
        WhiteoutError::Security(SecurityError::PathTraversal { .. }) => {
            eprintln!("  {} Ensure file paths are within the project directory", "└".bright_cyan());
        }
        WhiteoutError::Security(SecurityError::InsufficientPermissions { path }) => {
            eprintln!("  {} Check file permissions for {:?}", "└".bright_cyan(), path);
            eprintln!("    Try: chmod 644 {:?}", path);
        }
        WhiteoutError::Storage(StorageError::Locked) => {
            eprintln!("  {} Another process may be accessing the storage", "└".bright_cyan());
            eprintln!("    Wait a moment and try again");
        }
        WhiteoutError::Parse(ParseError::InvalidSyntax { line, .. }) => {
            eprintln!("  {} Check syntax at line {}", "└".bright_cyan(), line);
            eprintln!("    Expected format: // @whiteout: \"replacement\"");
        }
        WhiteoutError::Git(_) => {
            eprintln!("  {} Ensure you're in a Git repository", "└".bright_cyan());
            eprintln!("    Run: git init");
        }
        _ => {
            eprintln!("  {} Run with --verbose for more details", "└".bright_black(), );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = ParseError::InvalidSyntax {
            line: 42,
            message: "Missing closing bracket".to_string(),
        };
        
        let whiteout_error = WhiteoutError::Parse(error);
        let display = format!("{}", whiteout_error);
        
        assert!(display.contains("line 42"));
        assert!(display.contains("Missing closing bracket"));
    }
    
    #[test]
    fn test_error_context() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let result: Result<(), io::Error> = Err(io_error);
        
        let contextualized = result.context("Failed to open config file");
        assert!(contextualized.is_err());
        
        let error = contextualized.unwrap_err();
        let display = format!("{}", error);
        assert!(display.contains("Failed to open config file"));
    }
    
    #[test]
    fn test_security_error() {
        let error = SecurityError::PathTraversal {
            path: PathBuf::from("../../../etc/passwd"),
        };
        
        let whiteout_error = WhiteoutError::Security(error);
        let display = format!("{}", whiteout_error);
        
        assert!(display.contains("Path traversal"));
        assert!(display.contains("etc/passwd"));
    }
}