use whiteout::Whiteout;
use tempfile::TempDir;
use std::path::Path;

#[test]
fn test_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = "";
    let file_path = Path::new("empty.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    assert_eq!(cleaned, "");
    
    let smudged = whiteout.smudge(&cleaned, file_path).unwrap();
    assert_eq!(smudged, "");
}

#[test]
fn test_very_long_secret() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let long_secret = "x".repeat(10000);
    let content = format!(r#"const KEY = "{}"; // @whiteout: "REDACTED""#, long_secret);
    let file_path = Path::new("long.rs");
    
    let cleaned = whiteout.clean(&content, file_path).unwrap();
    assert!(!cleaned.contains(&long_secret));
    assert!(cleaned.contains("REDACTED"));
    
    let smudged = whiteout.smudge(&cleaned, file_path).unwrap();
    assert!(smudged.contains(&long_secret));
}

#[test]
fn test_unicode_in_secrets() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = r#"const SECRET = "密码🔐秘密"; // @whiteout: "HIDDEN""#;
    let file_path = Path::new("unicode.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    assert!(!cleaned.contains("密码"));
    assert!(!cleaned.contains("🔐"));
    assert!(cleaned.contains("HIDDEN"));
    
    let smudged = whiteout.smudge(&cleaned, file_path).unwrap();
    assert!(smudged.contains("密码🔐秘密"));
}

#[test]
fn test_special_characters_in_secret() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = r#"const KEY = "a!@#$%^&*()_+-=[]{}|;':\",./<>?"; // @whiteout: "SAFE""#;
    let file_path = Path::new("special.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    assert!(!cleaned.contains("a!@#$%^&*()"));
    assert!(cleaned.contains("SAFE"));
}

#[test]
fn test_nested_quotes() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = r#"const JSON = "{\"key\": \"secret\"}"; // @whiteout: "{\"key\": \"REDACTED\"}""#;
    let file_path = Path::new("json.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    assert!(!cleaned.contains("secret"));
    assert!(cleaned.contains("REDACTED"));
}

#[test]
fn test_multiple_decorations_same_line() {
    // This should NOT be supported - one decoration per line
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = r#"const A = "secret1"; // @whiteout: "SAFE1" const B = "secret2"; // @whiteout: "SAFE2""#;
    let file_path = Path::new("multi.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    // Should only process the first decoration
    assert!(!cleaned.contains("secret1"));
    // Second one might not be processed correctly
}

#[test]
fn test_decoration_without_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    // Missing replacement value
    let content = r#"const KEY = "secret"; // @whiteout:"#;
    let file_path = Path::new("invalid.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    // Should handle gracefully - no transformation
    assert_eq!(cleaned, content);
}

#[test]
fn test_block_with_no_end() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = r#"
// @whiteout-start
const SECRET = "value";
// Missing @whiteout-end
const OTHER = "data";
"#;
    let file_path = Path::new("noend.rs");
    
    let cleaned = whiteout.clean(content, file_path).unwrap();
    // Should handle gracefully
    assert_eq!(cleaned, content);
}

#[test]
fn test_concurrent_access() {
    use std::thread;
    
    // Create separate instances for each thread to avoid file locking issues
    let mut handles = vec![];
    
    for i in 0..10 {
        let handle = thread::spawn(move || {
            // Each thread gets its own temp directory and whiteout instance
            let temp_dir = TempDir::new().unwrap();
            let whiteout = Whiteout::init(temp_dir.path()).unwrap();
            
            let content = format!(r#"const KEY{} = "secret{}"; // @whiteout: "SAFE{}""#, i, i, i);
            let file_name = format!("file{}.rs", i);
            let file_path = Path::new(&file_name);
            
            let cleaned = whiteout.clean(&content, file_path).unwrap();
            assert!(!cleaned.contains(&format!("secret{}", i)));
            assert!(cleaned.contains(&format!("SAFE{}", i)));
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_real_world_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let whiteout = Whiteout::init(temp_dir.path()).unwrap();
    
    let content = r#"
{
  "database": {
    "host": "localhost",
    "port": 5432,
    "username": "admin",
    "password": "super-secret-password", // @whiteout: "REDACTED"
    "database": "myapp"
  },
  "api": {
    "key": "sk-proj-abc123def456", // @whiteout: process.env.API_KEY
    "endpoint": "https://[[dev.internal:8080||api.production.com]]/v2" // @whiteout-partial
  },
  "debug": true, // @whiteout: false
  "logLevel": "trace" // @whiteout: "error"
}
"#;
    
    let file_path = Path::new("config.json");
    let cleaned = whiteout.clean(content, file_path).unwrap();
    
    // Verify all secrets are removed
    assert!(!cleaned.contains("super-secret-password"));
    assert!(!cleaned.contains("sk-proj-abc123def456"));
    assert!(!cleaned.contains("dev.internal:8080"));
    
    // Verify safe values are present (with decorations preserved for smudge)
    assert!(cleaned.contains("REDACTED"));
    assert!(cleaned.contains("process.env.API_KEY"));
    assert!(cleaned.contains("api.production.com"));
    assert!(cleaned.contains("false"));
    assert!(cleaned.contains("\"error\""));
    // Decorations are preserved for smudge to work
    assert!(cleaned.contains("@whiteout"));
    
    // Verify restoration
    let smudged = whiteout.smudge(&cleaned, file_path).unwrap();
    assert!(smudged.contains("super-secret-password"));
    assert!(smudged.contains("sk-proj-abc123def456"));
}