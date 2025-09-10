use anyhow::{Context, Result};
use git2::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Comprehensive end-to-end test that verifies Whiteout works correctly with Git
/// This test creates a real Git repository, commits files with secrets, and verifies
/// that secrets never reach Git history while remaining in the working directory.
#[test]
fn test_end_to_end_git_integration() -> Result<()> {
    // Step 1: Build whiteout binary
    let whiteout_bin = build_whiteout()?;
    println!("✓ Built whiteout binary at: {}", whiteout_bin.display());

    // Step 2: Create test repository
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    let repo = Repository::init(repo_path)?;
    println!("✓ Created test repository at: {}", repo_path.display());

    // Step 3: Configure Git user
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;
    
    // Step 4: Initialize whiteout
    initialize_whiteout(&whiteout_bin, repo_path)?;
    
    // Step 5: Configure Git filters
    config.set_str("filter.whiteout.clean", &format!("{} clean", whiteout_bin.display()))?;
    config.set_str("filter.whiteout.smudge", &format!("{} smudge", whiteout_bin.display()))?;
    config.set_str("filter.whiteout.required", "true")?;
    println!("✓ Configured Git filters");

    // Step 6: Create comprehensive test file with all decoration types
    let test_file_path = repo_path.join("test_secrets.js");
    let test_content = create_test_file_content();
    fs::write(&test_file_path, &test_content)?;
    println!("✓ Created test file with all decoration types");

    // Step 7: Stage and commit the file using actual Git commands to trigger filters
    // We must use git commands (not git2 library) to ensure filters are applied
    let output = Command::new("git")
        .args(&["add", "test_secrets.js"])
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to stage file: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    let output = Command::new("git")
        .args(&["commit", "-m", "Test commit with secrets"])
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to commit: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    println!("✓ Committed file to Git");

    // Step 8: Get the committed content directly from Git
    // Get the HEAD commit
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let tree = commit.tree()?;
    let entry = tree.get_path(Path::new("test_secrets.js"))?;
    let blob = repo.find_blob(entry.id())?;
    let committed_content = std::str::from_utf8(blob.content())
        .context("Failed to read committed content as UTF-8")?;
    
    println!("\n=== Verifying committed content ===");
    
    // Step 9: Verify secrets are NOT in the committed content
    let secrets_that_must_not_be_committed = vec![
        ("sk-proj-SUPER-SECRET-KEY-123456789", "API key"),
        ("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9", "Auth token"),
        ("admin123!@#", "Password"),
        ("enabled: true", "Debug enabled flag"),
        ("logLevel: \"trace\"", "Debug log level"),
        ("postgresql://admin:secretpass@localhost", "Database URL with password"),
        ("admin:pass123@dev.localhost:8080", "URL with embedded credentials"),
        ("secret-token@internal.dev", "Webhook token"),
        ("sk-live-xyz789", "Live API key"),
        ("staging.internal", "Internal staging URL"),
        ("username: \"developer\"", "Developer username"),
        ("password: \"dev123456\"", "Developer password"),
    ];
    
    for (secret, description) in &secrets_that_must_not_be_committed {
        assert!(
            !committed_content.contains(secret),
            "❌ CRITICAL: {} found in Git commit!\nSecret: '{}'\nCommitted content:\n{}",
            description, secret, committed_content
        );
        println!("  ✓ {} NOT in commit", description);
    }

    // Step 10: Verify safe replacements ARE in the committed content
    let safe_values_that_must_be_committed = vec![
        ("process.env.API_KEY", "API key replacement"),
        ("process.env.AUTH_TOKEN", "Auth token replacement"),
        ("REDACTED", "Password redaction"),
        ("enabled: false", "Production debug flag"),
        ("logLevel: \"error\"", "Production log level"),
        ("process.env.DATABASE_URL", "Database URL replacement"),
        ("api.production.com", "Production API URL"),
        ("webhook.example.com", "Production webhook URL"),
        ("credentials: null", "Null credentials"),
    ];
    
    for (value, description) in &safe_values_that_must_be_committed {
        assert!(
            committed_content.contains(value),
            "❌ ERROR: {} missing from commit!\nExpected: '{}'\nCommitted content:\n{}",
            description, value, committed_content
        );
        println!("  ✓ {} present in commit", description);
    }

    // Step 11: Verify non-decorated patterns were NOT transformed
    let non_decorated_patterns = vec![
        ("matrix[[row||col]]", "Non-decorated array pattern"),
        ("[[a-z]||[0-9]]", "Non-decorated regex pattern"),
    ];
    
    for (pattern, description) in &non_decorated_patterns {
        assert!(
            committed_content.contains(pattern),
            "❌ ERROR: {} was incorrectly transformed!\nPattern should remain: '{}'\nCommitted content:\n{}",
            description, pattern, committed_content
        );
        println!("  ✓ {} unchanged in commit", description);
    }

    println!("\n=== Verifying working directory ===");

    // Step 12: Verify working directory still has secrets
    let working_content = fs::read_to_string(&test_file_path)?;
    
    let secrets_that_must_be_in_working = vec![
        ("sk-proj-SUPER-SECRET-KEY-123456789", "API key"),
        ("admin123!@#", "Password"),
        ("enabled: true", "Debug flag"),
        ("admin:pass123@dev.localhost:8080", "URL credentials"),
    ];
    
    for (secret, description) in &secrets_that_must_be_in_working {
        assert!(
            working_content.contains(secret),
            "❌ ERROR: {} missing from working directory!\nSecret: '{}'",
            description, secret
        );
        println!("  ✓ {} present in working directory", description);
    }

    // Step 13: Test checkout restoration (delete and restore file)
    println!("\n=== Testing checkout restoration ===");
    fs::remove_file(&test_file_path)?;
    
    // Use git checkout to restore the file
    let output = Command::new("git")
        .args(&["checkout", "test_secrets.js"])
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to checkout file: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    let restored_content = fs::read_to_string(&test_file_path)?;
    assert!(
        restored_content.contains("sk-proj-SUPER-SECRET-KEY-123456789"),
        "❌ ERROR: Smudge filter failed to restore secrets after checkout!"
    );
    println!("  ✓ Secrets restored after checkout");

    // Step 14: Test with decorations preserved
    println!("\n=== Verifying decoration preservation ===");
    assert!(
        committed_content.contains("// @whiteout:"),
        "❌ ERROR: Inline decorations not preserved in commit!"
    );
    assert!(
        committed_content.contains("// @whiteout-start"),
        "❌ ERROR: Block decorations not preserved in commit!"
    );
    assert!(
        committed_content.contains("// @whiteout-partial"),
        "❌ ERROR: Partial decorations not preserved in commit!"
    );
    println!("  ✓ All decoration markers preserved in commit");

    println!("\n✅ ALL END-TO-END TESTS PASSED SUCCESSFULLY!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Summary:");
    println!("• Secrets are kept in working directory");
    println!("• Secrets are filtered from Git commits");
    println!("• Safe replacement values are committed");
    println!("• Secrets are restored on checkout");
    println!("• Non-decorated patterns are untouched");
    println!("• All decoration types work correctly");
    println!("• Decoration markers are preserved");

    Ok(())
}

/// Build the whiteout binary and return its path
fn build_whiteout() -> Result<PathBuf> {
    // Try to find existing binary first
    let possible_paths = vec![
        PathBuf::from("target/release/whiteout"),
        PathBuf::from("target/debug/whiteout"),
    ];
    
    for path in &possible_paths {
        if path.exists() {
            return Ok(path.canonicalize()?);
        }
    }
    
    // Build if not found
    let output = Command::new("cargo")
        .args(&["build", "--release"])
        .output()
        .context("Failed to run cargo build")?;
    
    if !output.status.success() {
        // Try debug build if release fails
        let output = Command::new("cargo")
            .args(&["build"])
            .output()
            .context("Failed to run cargo build")?;
        
        if !output.status.success() {
            anyhow::bail!("Failed to build whiteout: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        return Ok(PathBuf::from("target/debug/whiteout").canonicalize()?);
    }
    
    Ok(PathBuf::from("target/release/whiteout").canonicalize()?)
}

/// Initialize whiteout in the repository
fn initialize_whiteout(whiteout_bin: &Path, repo_path: &Path) -> Result<()> {
    let output = Command::new(whiteout_bin)
        .arg("init")
        .current_dir(repo_path)
        .output()
        .context("Failed to run whiteout init")?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to initialize whiteout: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Verify .gitattributes was created
    let gitattributes = repo_path.join(".gitattributes");
    assert!(gitattributes.exists(), ".gitattributes not created");
    
    let content = fs::read_to_string(&gitattributes)?;
    assert!(content.contains("filter=whiteout"), ".gitattributes not properly configured");
    
    Ok(())
}

/// Create comprehensive test file content with all decoration types
fn create_test_file_content() -> String {
    r#"// Test file with various secret decorations

// 1. Inline decoration - API keys and tokens
const API_KEY = "sk-proj-SUPER-SECRET-KEY-123456789"; // @whiteout: process.env.API_KEY
const AUTH_TOKEN = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; // @whiteout: process.env.AUTH_TOKEN
const PASSWORD = "admin123!@#"; // @whiteout: "REDACTED"

// 2. Block decoration - Debug configuration
// @whiteout-start
const DEBUG_CONFIG = {
    enabled: true,
    verbose: true,
    logLevel: "trace",
    dbUrl: "postgresql://admin:secretpass@localhost:5432/devdb"
};
// @whiteout-end
const DEBUG_CONFIG = {
    enabled: false,
    verbose: false,
    logLevel: "error",
    dbUrl: process.env.DATABASE_URL
};

// 3. Partial decoration - URLs with embedded credentials
const API_URL = "https://[[admin:pass123@dev.localhost:8080||api.production.com]]/v1/endpoint"; // @whiteout-partial
const WEBHOOK = "https://[[secret-token@internal.dev||webhook.example.com]]/notify"; // @whiteout-partial

// 4. Mixed decorations in JSON config
const config = {
    apiKey: "sk-live-xyz789", // @whiteout: process.env.LIVE_API_KEY
    endpoint: "https://[[staging.internal||api.example.com]]/graphql", // @whiteout-partial
    // @whiteout-start
    credentials: {
        username: "developer",
        password: "dev123456"
    },
    // @whiteout-end
    credentials: null,
};

// 5. Test that non-decorated patterns are NOT transformed
const normalArray = matrix[[row||col]];  // This should NOT be transformed (no decorator)
const regexPattern = "[[a-z]||[0-9]]";   // This should NOT be transformed (no decorator)
"#.to_string()
}