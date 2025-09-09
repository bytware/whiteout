use std::fs;
use std::path::Path;
use tempfile::TempDir;
use whiteout::Whiteout;

#[test]
fn test_actual_secret_removal() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    // Original content with a real secret
    let original = r#"
const API_KEY: &str = "sk-proj-REAL-SECRET-KEY-12345"; // @whiteout: "REDACTED"
const DATABASE_URL: &str = "postgresql://user:password@localhost/db"; // @whiteout: "postgresql://localhost/db"
"#;
    
    let file_path = Path::new("config.rs");
    
    // Apply clean filter (simulates what Git does on commit)
    let cleaned = whiteout.clean(original, file_path).unwrap();
    
    // CRITICAL TEST: Ensure secrets are NOT in cleaned content
    println!("Cleaned content:\n{}", cleaned);
    assert!(!cleaned.contains("sk-proj-REAL-SECRET-KEY-12345"), 
        "CRITICAL: Secret key still present in cleaned content!");
    assert!(!cleaned.contains("user:password"), 
        "CRITICAL: Database password still present in cleaned content!");
    
    // Verify decorations are preserved
    assert!(cleaned.contains("@whiteout:"), 
        "Decorations should be preserved");
    
    // Verify safe values are present
    assert!(cleaned.contains("REDACTED"));
    assert!(cleaned.contains("postgresql://localhost/db"));
}

#[test]
fn test_actual_secret_restoration() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let original = r#"let key = "my-secret-123"; // @whiteout: "ENV_KEY""#;
    let file_path = Path::new("test.js");
    
    // Clean to store the secret
    let cleaned = whiteout.clean(original, file_path).unwrap();
    println!("After clean: {}", cleaned);
    
    // Smudge to restore (simulates Git checkout)
    let restored = whiteout.smudge(&cleaned, file_path).unwrap();
    println!("After smudge: {}", restored);
    
    // CRITICAL TEST: Secret should be restored
    assert!(restored.contains("my-secret-123"), 
        "Secret should be restored after smudge!");
}

#[test]
fn test_no_decoration_passthrough() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    // Content without decorations should pass through unchanged
    let content = r#"const NORMAL_CODE = "this is fine";"#;
    let file_path = Path::new("normal.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    assert_eq!(content, cleaned, 
        "Content without decorations should not be modified");
}

#[test]
fn test_malformed_decoration_handling() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    // Malformed decoration
    let content = r#"let key = "secret"; // @whiteout"#;  // Missing replacement value
    let file_path = Path::new("bad.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    // Should not crash, should handle gracefully
    assert_eq!(content, cleaned);
}

#[test]
fn test_multiple_secrets_in_file() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let original = r#"
const API_KEY = "secret1"; // @whiteout: "REDACTED1"
const TOKEN = "secret2"; // @whiteout: "REDACTED2"
const PASSWORD = "secret3"; // @whiteout: "REDACTED3"
"#;
    
    let file_path = Path::new("multi.rs");
    let cleaned = whiteout.clean(original, file_path).unwrap();
    
    // None of the secrets should be present
    assert!(!cleaned.contains("secret1"));
    assert!(!cleaned.contains("secret2"));
    assert!(!cleaned.contains("secret3"));
    
    // All safe values should be present
    assert!(cleaned.contains("REDACTED1"));
    assert!(cleaned.contains("REDACTED2"));
    assert!(cleaned.contains("REDACTED3"));
}

#[test]
fn test_partial_decoration() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let original = r#"let url = "https://[[admin:password123@localhost||example.com]]/api";"#;
    let file_path = Path::new("url.js");
    
    let cleaned = whiteout.clean(original, file_path).unwrap();
    println!("Partial cleaned: {}", cleaned);
    
    assert!(!cleaned.contains("admin:password123"), 
        "Credentials should be removed from URL");
    assert!(cleaned.contains("example.com"), 
        "Safe domain should be present");
}