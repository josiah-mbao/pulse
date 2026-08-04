mod tui;

use clap::Parser;
use pulse::cli::commands::Commands;
use pulse::cli::status::run_status;
use pulse::cli::top::run_top;
use tui::app::run_app;

#[derive(Parser)]
#[command(name = "pulse", about = "Linux system observability TUI", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Status) => run_status(),
        Some(Commands::Top) => run_top(),
        None => {
            if let Err(e) = run_app() {
                eprintln!("Error running Pulse: {}", e);
                std::process::exit(1);
            }
        }
    }
}
