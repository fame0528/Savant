use anyhow::Result;
use colored::*;
use std::path::PathBuf;

pub async fn cmd_test_skill(skill_path: &str, input: &str, timeout_secs: u64) -> Result<()> {
    println!("{}", "=== Savant Skill Tester ===".cyan().bold());
    println!();

    let path = PathBuf::from(skill_path);
    if !path.exists() {
        eprintln!(
            "{} Skill file not found: {}",
            "Error:".red().bold(),
            skill_path
        );
        std::process::exit(1);
    }

    println!("{} Loading skill from: {}", "→".bright_black(), skill_path);
    let mut registry = savant_skills::parser::SkillRegistry::new();
    registry
        .load_skill_from_file(&path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load skill: {}", e))?;

    let skill_name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("{} Skill name: {}", "→".bright_black(), skill_name.green());
    println!();

    let input_value: serde_json::Value = serde_json::from_str(input).unwrap_or_else(|_| {
        let mut m = serde_json::Map::new();
        m.insert(
            "raw".to_string(),
            serde_json::Value::String(input.to_string()),
        );
        serde_json::Value::Object(m)
    });

    println!(
        "{} Input: {}",
        "→".bright_black(),
        serde_json::to_string_pretty(&input_value).unwrap_or_default()
    );
    println!();

    if let Some(tool) = registry.tools.values().next() {
        println!("{} Executing skill...", "→".bright_black());

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tool.execute(input_value),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!();
                println!("{}", "=== Output ===".green().bold());
                println!("{}", output);
                println!();
                println!("{}", "✅ Skill test PASSED".green().bold());
            }
            Ok(Err(e)) => {
                println!();
                println!("{}", "=== Error ===".red().bold());
                println!("{}", e);
                println!();
                println!("{}", "❌ Skill test FAILED".red().bold());
                std::process::exit(1);
            }
            Err(_) => {
                println!();
                println!(
                    "{}",
                    format!("❌ Skill test TIMED OUT after {}s", timeout_secs)
                        .red()
                        .bold()
                );
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("{} No tools found in skill manifest", "Error:".red().bold());
        std::process::exit(1);
    }

    Ok(())
}
