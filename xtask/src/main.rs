use std::sync::LazyLock;

use clap::{Parser, Subcommand};
use eden_utils::Result;
use nu_ansi_term::{Color, Style};

mod docker;
mod generate;

#[derive(Parser)]
#[command(version, author, long_about)]
struct TaskArgs {
    /// This option turns logging on.
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    subcommand: TaskSubcommand,
}

#[derive(Subcommand)]
enum TaskSubcommand {
    /// Docker-related utility tasks for the Eden project.
    /// (Docker installation is required)
    ///
    /// If you wish to run this command, you're required to install Docker
    /// in your system/dev environment to run this command.
    Docker(self::docker::DockerArgs),

    /// Generates something.
    Generate(self::generate::GenerateArgs),
}

fn main() -> Result<()> {
    eden_utils::Error::init();
    eden_utils::env::init();

    let args = TaskArgs::parse();
    let level = match args.debug {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };

    pretty_env_logger::formatted_timed_builder()
        .filter_level(level)
        .format_timestamp_millis()
        .init();

    match args.subcommand {
        TaskSubcommand::Docker(cmd) => self::docker::run(&cmd),
        TaskSubcommand::Generate(cmd) => self::generate::run(&cmd),
    }
}

const DONE_STYLE: LazyLock<Style> = LazyLock::new(|| Style::new().fg(Color::LightGreen).bold());
const ERROR_STYLE: LazyLock<Style> = LazyLock::new(|| Style::new().fg(Color::LightRed).bold());
