use clap::{Args, Subcommand};
use eden_utils::Result;

mod settings;

#[derive(Debug, Args)]
pub(super) struct GenerateCmd {
    #[clap(subcommand)]
    cmd: Inner,
}

#[derive(Debug, Subcommand)]
enum Inner {
    Settings,
}

impl GenerateCmd {
    pub fn run(&self) -> Result<()> {
        match self.cmd {
            Inner::Settings => self::settings::run(),
        }
    }
}
