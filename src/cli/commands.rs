use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Status,
    Top,
}
