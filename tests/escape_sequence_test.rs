use whiteout::Whiteout;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_escaped_decorations_in_documentation() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let documentation = r#"
# Whiteout Documentation

Here's how to use decorations:

## Inline Example
Use `\@whiteout:` to mark inline secrets:
```
const key = "secret"; // \@whiteout: "REDACTED"
```

## Block Example
Use `\@whiteout-start` and `\@whiteout-end`:
```
// \@whiteout-start
secret code here
// \@whiteout-end
```

## Simple Example
Just use `\@whiteout` on its own line:
```
\@whiteout
This would be hidden if not escaped
```

Note: The backslash escapes the decorator so it appears in documentation.
"#;

    let file_path = Path::new("DOCS.md");
    let cleaned = whiteout.clean(documentation, file_path)?;
    
    // Escaped decorations should remain as-is (without the backslash)
    assert_eq!(cleaned, documentation);
    
    // The escaped decorations should still appear in the output
    assert!(cleaned.contains(r"\@whiteout"));
    assert!(cleaned.contains(r"\@whiteout-start"));
    assert!(cleaned.contains(r"\@whiteout-end"));
    
    Ok(())
}

#[test]
fn test_mixed_escaped_and_real_decorations() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let mixed_content = r#"
# Documentation

This shows how to use decorations: `\@whiteout:`

But this is a real secret:
password = "secret123"  # @whiteout: "getpass()"

More docs about `\@whiteout-start` and `\@whiteout-end`

# @whiteout-start
This is actually hidden
# @whiteout-end
This stays

Final note about `\@whiteout` usage.
"#;

    let file_path = Path::new("mixed.md");
    let cleaned = whiteout.clean(mixed_content, file_path)?;
    
    // Documentation with escaped decorators should remain
    assert!(cleaned.contains(r"\@whiteout:"));
    assert!(cleaned.contains(r"\@whiteout-start"));
    assert!(cleaned.contains(r"\@whiteout-end"));
    assert!(cleaned.contains(r"\@whiteout"));
    
    // Real decorations should be processed
    assert!(cleaned.contains("\"getpass()\""));
    assert!(!cleaned.contains("secret123"));
    assert!(!cleaned.contains("This is actually hidden"));
    assert!(cleaned.contains("This stays"));
    
    Ok(())
}

#[test]
fn test_escape_sequence_preservation() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    // Test that backslash-escaped decorators are preserved literally
    let content = r#"
Regular line
\@whiteout
This line has escaped decorator and should stay

@whiteout
This line has real decorator and should be hidden

More content
"#;

    let file_path = Path::new("test.txt");
    let cleaned = whiteout.clean(content, file_path)?;
    
    // Escaped decorator line should remain
    assert!(cleaned.contains(r"\@whiteout"));
    assert!(cleaned.contains("This line has escaped decorator"));
    
    // Real decorator should be processed
    assert!(!cleaned.contains("This line has real decorator"));
    
    // But the escaped one should not trigger hiding
    assert!(cleaned.contains("Regular line"));
    assert!(cleaned.contains("More content"));
    
    Ok(())
}