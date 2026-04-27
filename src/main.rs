use clap::Parser;

mod cli;
use cli::commands::Commands;
use cli::status::run_status;
use tui::app::run_app;

#[derive(Parser)]
#[command(name = "pulse")]
#[command(about = "System observability tool for Linux")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    run_app().unwrap();
}
