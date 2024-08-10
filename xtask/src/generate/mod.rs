use clap::{Args, Subcommand};
use eden_utils::Result;

mod settings;

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[clap(subcommand)]
    subcommand: GenerateSubcommand,
}

#[derive(Debug, Subcommand)]
enum GenerateSubcommand {
    /// Generates the entire documentation of settings in every
    /// and saves it in `config/eden.example.toml`.
    Settings,
}

pub fn run(args: &GenerateArgs) -> Result<()> {
    match args.subcommand {
        GenerateSubcommand::Settings => self::settings::run(),
    }
}
