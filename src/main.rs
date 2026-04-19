use clap::Parser;

mod cli;
use cli::commands::Commands;
use cli::status::run_status;

#[derive(Parser)]
#[command(name = "pulse")]
#[command(about = "System observability tool for Linux")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            run_status();
        }
    }
}
