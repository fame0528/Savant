use anyhow::Result;
use colored::*;
use savant_agent::orchestration::ignition::IgnitionService;

pub fn print_splash() {
    let logo = r#"
    ███████╗ █████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗
    ██╔════╝██╔══██║██║   ██║██╔══██╗████╗  ██║╚══██╔══╝
    ███████╗███████║██║   ██║███████║██╔██╗ ██║   ██║   
    ╚════██║██╔══██║╚██╗ ██╔╝██╔══██║██║╚██╗██║   ██║   
    ███████║██║  ██║ ╚████╔╝ ██║  ██║██║ ╚████║   ██║   
    ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝   
    "#;
    println!("{}", logo.cyan().bold());
    println!(
        "{}",
        "      >> AUTONOMOUS AGENT SWARM ORCHESTRATOR <<"
            .bright_black()
            .italic()
    );
    println!(
        "{}",
        "            ONE MIND. A THOUSAND FACES.".cyan().bold()
    );
    println!(
        "{}",
        "====================================================".bright_blue()
    );
    println!(
        "{}",
        format!("   v{} PRODUCTION", env!("CARGO_PKG_VERSION"))
            .green()
            .bold()
    );

    let build_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let days = secs / 86400;
            let years = 1970 + days / 365;
            years.to_string()
        })
        .unwrap_or_else(|_| "unknown".to_string());
    println!(
        "{}",
        format!("   Build: {} (runtime)", build_time).bright_black()
    );

    println!(
        "{}",
        "====================================================".bright_blue()
    );
    println!();
}

pub fn print_phase(num: u8, desc: &str) {
    println!(
        "{} {} {}",
        format!("[PHASE {}]", num).blue().bold(),
        "→".bright_black(),
        desc.bright_white()
    );
}

pub async fn cmd_start(config_path: Option<String>) -> Result<()> {
    print_splash();
    print_phase(1, "Loading configuration");
    print_phase(2, "Igniting substrate");

    let _ignition = IgnitionService::ignite(config_path.as_deref()).await?;

    print_phase(3, "Substrate active");

    println!();
    println!(
        "{}",
        "----------------------------------------------------".bright_blue()
    );
    println!(
        "{} {}",
        "🚀 STATUS:".bright_cyan().bold(),
        "ACTIVE & PERSISTENT".green().bold()
    );
    println!(
        "{} {}",
        "📱 DASH:  ".bright_cyan().bold(),
        "http://localhost:3000".white().underline()
    );
    println!(
        "{} {}",
        "🔗 GATE:  ".bright_cyan().bold(),
        "ws://localhost:3000".white().underline()
    );
    println!(
        "{}",
        "----------------------------------------------------".bright_blue()
    );
    println!(
        "{}",
        "Press Ctrl+C to terminate the swarm"
            .bright_black()
            .italic()
    );
    println!();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!();
                tracing::info!("🛑 {}", "Received shutdown signal. Evacuating agents...".yellow());
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(3600)) => {
                tracing::debug!("Swarm pulse nominal...");
            }
        }
    }

    tracing::info!("✅ {}", "Savant shutdown complete".green());
    Ok(())
}
