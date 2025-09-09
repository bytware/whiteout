use whiteout::Whiteout;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("Whiteout Demo - Local-Only Code Decorations");
    println!("==========================================\n");

    let example_code = r#"
// Example 1: Inline decoration
let api_key = "sk-my-secret-key-12345"; // @whiteout: std::env::var("API_KEY").unwrap()

// Example 2: Block decoration
// @whiteout-start
const DEBUG: bool = true;
const LOG_LEVEL: &str = "trace";
const DB_URL: &str = "postgresql://localhost/dev_db";
// @whiteout-end
const DEBUG: bool = false;
const LOG_LEVEL: &str = "error";
const DB_URL: &str = std::env::var("DATABASE_URL").unwrap();

// Example 3: Partial decoration (requires @whiteout-partial)
let api_url = "https://[[dev.local:8080||api.production.com]]/v1/endpoint"; // @whiteout-partial
let config = Config {
    host: "[[localhost||example.com]]", // @whiteout-partial
    port: [[8080||443]], // @whiteout-partial
};
"#;

    println!("Original Code (with decorations):");
    println!("{}", example_code);
    println!("\n{}", "=".repeat(60));

    let whiteout = Whiteout::new(".")?;
    let file_path = Path::new("example.rs");

    println!("\nApplying CLEAN filter (for Git commit):");
    println!("{}", "-".repeat(40));
    let cleaned = whiteout.clean(example_code, file_path)?;
    println!("{}", cleaned);

    println!("\n{}", "=".repeat(60));
    println!("\nApplying SMUDGE filter (for local checkout):");
    println!("{}", "-".repeat(40));
    let smudged = whiteout.smudge(&cleaned, file_path)?;
    println!("{}", smudged);

    Ok(())
}