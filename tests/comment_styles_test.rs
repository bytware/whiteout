use whiteout::Whiteout;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_python_comment_style() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let python_code = r#"
api_key = "sk-12345"  # @whiteout: "ENV_VAR"

# @whiteout-start
DEBUG = True
# @whiteout-end
DEBUG = False
"#;

    let file_path = Path::new("test.py");
    let cleaned = whiteout.clean(python_code, file_path)?;
    
    // Check that decorations are removed and only committed values remain
    assert!(cleaned.contains("\"ENV_VAR\""));
    assert!(!cleaned.contains("sk-12345"));
    assert!(cleaned.contains("@whiteout"));  // Decorations preserved
    assert!(cleaned.contains("DEBUG = False"));
    assert!(!cleaned.contains("DEBUG = True"));
    
    Ok(())
}

#[test]
fn test_sql_comment_style() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let sql_code = r#"
SELECT * FROM users WHERE api_key = 'sk-12345'; -- @whiteout: 'REDACTED'

-- @whiteout-start
INSERT INTO debug_logs VALUES ('sensitive data');
-- @whiteout-end
-- Production logging disabled
"#;

    let file_path = Path::new("test.sql");
    let cleaned = whiteout.clean(sql_code, file_path)?;
    
    assert!(cleaned.contains("'REDACTED'"));
    assert!(!cleaned.contains("sk-12345"));
    assert!(!cleaned.contains("sensitive data"));
    assert!(cleaned.contains("@whiteout"));  // Decorations preserved
    
    Ok(())
}

#[test]
fn test_markdown_simple_whiteout() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let markdown = r#"
# Documentation

Here's some public content.

@whiteout
This is secret information that shouldn't be committed.
It can span multiple lines.

Back to public documentation.
"#;

    let file_path = Path::new("README.md");
    let cleaned = whiteout.clean(markdown, file_path)?;
    
    assert!(cleaned.contains("# Documentation"));
    assert!(cleaned.contains("public content"));
    assert!(cleaned.contains("Back to public"));
    assert!(!cleaned.contains("secret information"));
    assert!(cleaned.contains("@whiteout"));  // Decorations preserved
    assert!(!cleaned.contains("multiple lines"));
    
    Ok(())
}

#[test]
fn test_mixed_comment_styles() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    // This might occur in a Dockerfile or shell script with inline Python
    let mixed_code = r#"
// JavaScript style
const apiKey = "sk-12345"; // @whiteout: "REDACTED"

# Python/Shell style
API_KEY="sk-67890"  # @whiteout: "ENV_VAR"

-- SQL style
SELECT * FROM keys WHERE key = 'sk-11111'; -- @whiteout: 'HIDDEN'
"#;

    let file_path = Path::new("mixed.txt");
    let cleaned = whiteout.clean(mixed_code, file_path)?;
    
    // All three comment styles should work
    assert!(cleaned.contains("\"REDACTED\""));
    assert!(cleaned.contains("\"ENV_VAR\""));
    assert!(cleaned.contains("'HIDDEN'"));
    
    // No secrets should remain
    assert!(!cleaned.contains("sk-12345"));
    assert!(!cleaned.contains("sk-67890"));
    assert!(!cleaned.contains("sk-11111"));
    
    // Decorations should be preserved
    assert!(cleaned.contains("@whiteout"));  // Decorations preserved
    
    Ok(())
}

#[test]
fn test_edge_case_no_decorations() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let plain_code = r#"
This is just normal code without any decorations.
No secrets here.
Just regular content.
"#;

    let file_path = Path::new("plain.txt");
    let cleaned = whiteout.clean(plain_code, file_path)?;
    
    // Should return unchanged
    assert_eq!(cleaned, plain_code);
    
    Ok(())
}