use crate::error::{SecurityError, WhiteoutError, WhiteoutResult};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

// Security patterns to detect malicious input
static COMMAND_INJECTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[;&|`$<>]|\$\(|\)|&&|\|\|").expect("Failed to compile injection pattern")
});

static PATH_TRAVERSAL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\.\./|\.\.\\|~|%2e%2e").expect("Failed to compile traversal pattern")
});

static CONTROL_CHAR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").expect("Failed to compile control pattern")
});

static SUSPICIOUS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(exec|eval|system|shell|cmd|powershell|bash|sh\s)").expect("Failed to compile suspicious pattern")
});

/// Input validation for security
pub struct InputValidator;

impl InputValidator {
    /// Validate file path for security issues
    pub fn validate_path<P: AsRef<Path>>(path: P, base_dir: &Path) -> WhiteoutResult<PathBuf> {
        let path = path.as_ref();
        
        // Check for path traversal attempts
        let path_str = path.to_string_lossy();
        if PATH_TRAVERSAL_PATTERN.is_match(&path_str) {
            return Err(WhiteoutError::Security(SecurityError::PathTraversal {
                path: path.to_path_buf(),
            }));
        }
        
        // Resolve to canonical path
        let canonical = if path.exists() {
            path.canonicalize()
                .map_err(|e| WhiteoutError::Io(e))?
        } else {
            // For non-existent files, validate parent and construct path
            let parent = path.parent()
                .ok_or_else(|| WhiteoutError::InvalidInput(
                    "Invalid path: no parent directory".to_string()
                ))?;
            
            if !parent.exists() {
                return Err(WhiteoutError::InvalidInput(
                    format!("Parent directory does not exist: {:?}", parent)
                ));
            }
            
            let parent_canonical = parent.canonicalize()
                .map_err(|e| WhiteoutError::Io(e))?;
            
            let file_name = path.file_name()
                .ok_or_else(|| WhiteoutError::InvalidInput(
                    "Invalid path: no file name".to_string()
                ))?;
            
            // Validate file name
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.starts_with('.') && file_name_str != ".gitignore" {
                // Allow .gitignore but be cautious about other hidden files
                if !file_name_str.starts_with(".whiteout") {
                    return Err(WhiteoutError::Security(SecurityError::SuspiciousPattern {
                        pattern: format!("Hidden file: {}", file_name_str),
                    }));
                }
            }
            
            parent_canonical.join(file_name)
        };
        
        // Ensure path is within base directory
        let base_canonical = base_dir.canonicalize()
            .map_err(|e| WhiteoutError::Io(e))?;
        
        if !canonical.starts_with(&base_canonical) {
            return Err(WhiteoutError::Security(SecurityError::PathTraversal {
                path: canonical.clone(),
            }));
        }
        
        Ok(canonical)
    }
    
    /// Validate decoration content for security issues
    pub fn validate_decoration(content: &str) -> WhiteoutResult<()> {
        const MAX_LENGTH: usize = 10000;
        
        // Check length
        if content.len() > MAX_LENGTH {
            return Err(WhiteoutError::InvalidInput(
                format!("Decoration content exceeds maximum length of {} bytes", MAX_LENGTH)
            ));
        }
        
        // Check for null bytes
        if content.contains('\0') {
            return Err(WhiteoutError::Security(SecurityError::SuspiciousPattern {
                pattern: "Null byte in decoration".to_string(),
            }));
        }
        
        // Check for control characters (except newline and tab)
        if CONTROL_CHAR_PATTERN.is_match(content) {
            return Err(WhiteoutError::Security(SecurityError::SuspiciousPattern {
                pattern: "Control characters in decoration".to_string(),
            }));
        }
        
        // Check for command injection patterns
        if COMMAND_INJECTION_PATTERN.is_match(content) {
            // Allow these in string literals but warn
            if !content.starts_with('"') || !content.ends_with('"') {
                return Err(WhiteoutError::Security(SecurityError::CommandInjection {
                    command: content.to_string(),
                }));
            }
        }
        
        Ok(())
    }
    
    /// Validate replacement value
    pub fn validate_replacement(value: &str) -> WhiteoutResult<String> {
        // Sanitize the replacement value
        let sanitized = value
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .take(1000) // Limit length
            .collect::<String>();
        
        // Check for suspicious patterns
        if SUSPICIOUS_PATTERN.is_match(&sanitized) {
            return Err(WhiteoutError::Security(SecurityError::SuspiciousPattern {
                pattern: "Potentially dangerous command in replacement".to_string(),
            }));
        }
        
        Ok(sanitized)
    }
    
    /// Validate line number or range
    pub fn validate_line_spec(spec: &str, max_lines: usize) -> WhiteoutResult<LineSpec> {
        if spec.contains('-') {
            // Range format: start-end
            let parts: Vec<&str> = spec.split('-').collect();
            if parts.len() != 2 {
                return Err(WhiteoutError::InvalidInput(
                    "Invalid line range format. Use: start-end".to_string()
                ));
            }
            
            let start: usize = parts[0].parse()
                .map_err(|_| WhiteoutError::InvalidInput(
                    format!("Invalid start line number: {}", parts[0])
                ))?;
            
            let end: usize = parts[1].parse()
                .map_err(|_| WhiteoutError::InvalidInput(
                    format!("Invalid end line number: {}", parts[1])
                ))?;
            
            if start < 1 || end > max_lines || start > end {
                return Err(WhiteoutError::InvalidInput(
                    format!("Invalid line range: {}-{} (file has {} lines)", start, end, max_lines)
                ));
            }
            
            Ok(LineSpec::Range { start, end })
        } else {
            // Single line
            let line: usize = spec.parse()
                .map_err(|_| WhiteoutError::InvalidInput(
                    format!("Invalid line number: {}", spec)
                ))?;
            
            if line < 1 || line > max_lines {
                return Err(WhiteoutError::InvalidInput(
                    format!("Line {} out of range (file has {} lines)", line, max_lines)
                ));
            }
            
            Ok(LineSpec::Single(line))
        }
    }
    
    /// Validate Git command arguments
    pub fn validate_git_args(args: &[String]) -> WhiteoutResult<()> {
        for arg in args {
            // Check for command injection
            if COMMAND_INJECTION_PATTERN.is_match(arg) {
                return Err(WhiteoutError::Security(SecurityError::CommandInjection {
                    command: arg.clone(),
                }));
            }
            
            // Check for path traversal in file arguments
            if arg.starts_with("../") || arg.contains("/../") {
                return Err(WhiteoutError::Security(SecurityError::PathTraversal {
                    path: PathBuf::from(arg),
                }));
            }
        }
        
        Ok(())
    }
    
    /// Validate configuration key
    pub fn validate_config_key(key: &str) -> WhiteoutResult<()> {
        // Only allow alphanumeric and underscore
        if !key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Err(WhiteoutError::InvalidInput(
                format!("Invalid configuration key: {}. Use only alphanumeric, underscore, and dot", key)
            ));
        }
        
        // Check length
        if key.len() > 100 {
            return Err(WhiteoutError::InvalidInput(
                "Configuration key too long (max 100 characters)".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate configuration value
    pub fn validate_config_value(value: &str) -> WhiteoutResult<()> {
        // Check length
        if value.len() > 1000 {
            return Err(WhiteoutError::InvalidInput(
                "Configuration value too long (max 1000 characters)".to_string()
            ));
        }
        
        // Check for control characters
        if CONTROL_CHAR_PATTERN.is_match(value) {
            return Err(WhiteoutError::InvalidInput(
                "Control characters not allowed in configuration values".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Line specification after validation
#[derive(Debug, Clone)]
pub enum LineSpec {
    Single(usize),
    Range { start: usize, end: usize },
}

/// File permissions validator
pub struct PermissionValidator;

impl PermissionValidator {
    /// Check if file has secure permissions
    #[cfg(unix)]
    pub fn check_file_permissions(path: &Path) -> WhiteoutResult<()> {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        
        let metadata = fs::metadata(path)
            .map_err(|e| WhiteoutError::Io(e))?;
        
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        
        // Check for world-writable files
        if mode & 0o002 != 0 {
            return Err(WhiteoutError::Security(SecurityError::InsufficientPermissions {
                path: path.to_path_buf(),
            }));
        }
        
        // Check for world-readable sensitive files
        if path.file_name() == Some(std::ffi::OsStr::new(".salt")) && mode & 0o004 != 0 {
            return Err(WhiteoutError::Security(SecurityError::InsufficientPermissions {
                path: path.to_path_buf(),
            }));
        }
        
        Ok(())
    }
    
    #[cfg(not(unix))]
    pub fn check_file_permissions(_path: &Path) -> WhiteoutResult<()> {
        // Windows permission checking would go here
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_path_validation() {
        let base = Path::new("/home/user/project");
        
        // Valid path
        let result = InputValidator::validate_path(
            Path::new("/home/user/project/src/main.rs"),
            base
        );
        assert!(result.is_ok());
        
        // Path traversal attempt
        let result = InputValidator::validate_path(
            Path::new("/home/user/project/../../../etc/passwd"),
            base
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_decoration_validation() {
        // Valid decoration
        assert!(InputValidator::validate_decoration("REDACTED").is_ok());
        
        // Null byte
        assert!(InputValidator::validate_decoration("test\0").is_err());
        
        // Too long
        let long_content = "x".repeat(10001);
        assert!(InputValidator::validate_decoration(&long_content).is_err());
    }
    
    #[test]
    fn test_line_spec_validation() {
        // Single line
        let spec = InputValidator::validate_line_spec("5", 10).unwrap();
        assert!(matches!(spec, LineSpec::Single(5)));
        
        // Range
        let spec = InputValidator::validate_line_spec("3-7", 10).unwrap();
        assert!(matches!(spec, LineSpec::Range { start: 3, end: 7 }));
        
        // Out of range
        assert!(InputValidator::validate_line_spec("15", 10).is_err());
        
        // Invalid range
        assert!(InputValidator::validate_line_spec("7-3", 10).is_err());
    }
    
    #[test]
    fn test_config_validation() {
        // Valid key
        assert!(InputValidator::validate_config_key("encryption.enabled").is_ok());
        
        // Invalid key with special chars
        assert!(InputValidator::validate_config_key("key$value").is_err());
        
        // Valid value
        assert!(InputValidator::validate_config_value("true").is_ok());
        
        // Value with control chars
        assert!(InputValidator::validate_config_value("test\x00").is_err());
    }
}