use whiteout::Whiteout;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_full_workflow() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let original_code = r#"
let api_key = "sk-12345"; // @whiteout: load_from_env()

// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;

let url = "https://[[localhost||api.example.com]]/v1";
"#;

    let file_path = Path::new("test.rs");
    
    let cleaned = whiteout.clean(original_code, file_path)?;
    
    assert!(cleaned.contains("load_from_env()"));
    assert!(!cleaned.contains("sk-12345"));
    assert!(cleaned.contains("const DEBUG = false"));
    assert!(!cleaned.contains("const DEBUG = true"));
    assert!(cleaned.contains("api.example.com"));
    assert!(!cleaned.contains("localhost"));
    
    let smudged = whiteout.smudge(&cleaned, file_path)?;
    
    assert!(smudged.contains("sk-12345"));
    assert!(smudged.contains("const DEBUG = true"));
    assert!(smudged.contains("localhost"));
    
    Ok(())
}

#[test]
fn test_nested_decorations() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let code = r#"
// @whiteout-start
let config = Config {
    api_key: "sk-12345",
    debug: true,
};
// @whiteout-end
let config = Config {
    api_key: env::var("API_KEY")?,
    debug: false,
};
"#;

    let file_path = Path::new("config.rs");
    let cleaned = whiteout.clean(code, file_path)?;
    
    assert!(cleaned.contains(r#"api_key: env::var("API_KEY")?"#));
    assert!(cleaned.contains("debug: false"));
    assert!(!cleaned.contains("sk-12345"));
    assert!(!cleaned.contains("debug: true"));
    
    Ok(())
}

#[test]
fn test_multiple_partial_replacements() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let code = r#"let endpoints = ["[[http://localhost:8080||https://api.prod.com]]", "[[ws://localhost:9090||wss://ws.prod.com]]"];"#;
    
    let file_path = Path::new("endpoints.rs");
    let cleaned = whiteout.clean(code, file_path)?;
    
    assert!(cleaned.contains("https://api.prod.com"));
    assert!(cleaned.contains("wss://ws.prod.com"));
    assert!(!cleaned.contains("localhost"));
    
    let smudged = whiteout.smudge(&cleaned, file_path)?;
    
    assert!(smudged.contains("http://localhost:8080"));
    assert!(smudged.contains("ws://localhost:9090"));
    
    Ok(())
}