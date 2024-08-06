use clap::{Parser, Subcommand};

mod generate;

#[derive(Debug, Parser)]
struct CliArgs {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Generate(self::generate::GenerateCmd),
}

fn main() {
    let args = CliArgs::parse();
    if let Err(error) = match args.cmd {
        Command::Generate(cmd) => cmd.run(),
    } {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
