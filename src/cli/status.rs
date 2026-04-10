//! Status command - show active sessions, alerts, and incidents
//!
//! Usage: logpilot status

use clap::Args;

/// Show status of monitored sessions and active alerts
#[derive(Args, Clone)]
pub struct StatusArgs {
    /// Show detailed information
    #[arg(short, long)]
    pub detailed: bool,
    /// Filter by session name
    #[arg(short, long)]
    pub session: Option<String>,
}

/// Handle the status command
pub async fn handle(args: StatusArgs) -> anyhow::Result<()> {
    println!("{}", "=".repeat(60));
    println!("LogPilot Status");
    println!("{}", "=".repeat(60));

    // Session status (placeholder - would integrate with SessionRepository)
    println!("\n📊 Monitored Sessions");
    println!("  (Integration with SessionRepository needed for live data)");

    if let Some(ref session_name) = args.session {
        println!("  Filter: {}", session_name);
    }

    // Placeholder for session list
    println!("  No active sessions (capture not running)");
    println!("  Run 'logpilot watch <session>' to start monitoring");

    // Alert status (placeholder - would integrate with AlertRepository)
    println!("\n🚨 Active Alerts");
    println!("  (Integration with AlertRepository needed for live data)");
    println!("  No active alerts");

    // Incident status (placeholder - would integrate with IncidentRepository)
    println!("\n🔥 Active Incidents");
    println!("  (Integration with IncidentRepository needed for live data)");
    println!("  No active incidents");

    // Pattern status
    println!("\n📈 Detected Patterns");
    println!("  (Integration with PatternTracker needed for live data)");
    println!("  No patterns detected");

    if args.detailed {
        println!("\n📋 Detailed Information");
        println!("  Buffer Status: (not connected)");
        println!("  Persistence: (not configured)");
        println!("  MCP Server: (not running)");
    }

    println!("\n{}", "=".repeat(60));
    println!("Status command shows live data when watch is active.");
    println!("{}", "=".repeat(60));

    Ok(())
}
