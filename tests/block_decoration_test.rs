use whiteout::Whiteout;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_block_decoration_preservation() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let whiteout = Whiteout::init(temp_dir.path())?;
    
    let original = r#"// @whiteout-start
const DEBUG = true;
const LOG = "trace";
// @whiteout-end
const DEBUG = false;
const LOG = "error";"#;
    
    let file_path = Path::new("test.js");
    
    // Clean (for commit)
    let cleaned = whiteout.clean(original, file_path)?;
    println!("Original:\n{}", original);
    println!("\nCleaned:\n{}", cleaned);
    
    // Check decorations are PRESERVED (for smudge to work)
    assert!(cleaned.contains("@whiteout-start"), "Decoration @whiteout-start not preserved");
    assert!(cleaned.contains("@whiteout-end"), "Decoration @whiteout-end not preserved");
    
    // Check only safe values are present
    assert!(cleaned.contains("const DEBUG = false"), "Missing safe DEBUG value");
    assert!(cleaned.contains("const LOG = \"error\""), "Missing safe LOG value");
    assert!(!cleaned.contains("const DEBUG = true"), "Secret DEBUG value leaked!");
    assert!(!cleaned.contains("const LOG = \"trace\""), "Secret LOG value leaked!");
    
    // Now let's check storage
    let storage_path = temp_dir.path().join(".whiteout/local.toml");
    if storage_path.exists() {
        let storage_content = std::fs::read_to_string(&storage_path)?;
        println!("\nStorage content:\n{}", storage_content);
    }
    
    // With preserved decorations, smudge can restore from cleaned content
    println!("\nCalling smudge on cleaned content...");
    let smudged = whiteout.smudge(&cleaned, file_path)?;
    println!("\nSmudged:\n{}", smudged);
    
    // Check secrets are restored when decorations are present
    assert!(smudged.contains("const DEBUG = true"), "Secret DEBUG not restored");
    assert!(smudged.contains("const LOG = \"trace\""), "Secret LOG not restored");
    
    Ok(())
}