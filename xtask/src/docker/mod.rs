use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eden_utils::error::{exts::*, tags::Suggestion};
use eden_utils::Result;
use thiserror::Error;
use which::which;

mod build;

#[derive(Parser)]
pub struct DockerArgs {
    #[command(subcommand)]
    subcommand: DockerSubcommand,
}

#[derive(Subcommand)]
pub enum DockerSubcommand {
    /// Builds Eden docker image with `Dockerfile` located at
    /// the root directory of the Eden project repository.
    Build(self::build::BuildArgs),
}

pub fn run(args: &DockerArgs) -> Result<()> {
    let docker_path = get_docker_executable_path()?;
    match &args.subcommand {
        DockerSubcommand::Build(args) => self::build::run(docker_path, &args),
    }
}

fn get_docker_executable_path() -> Result<PathBuf> {
    which("docker")
        .into_typed_error()
        .change_context(DockerCmdError::NotInstalled)
        .attach(Suggestion::new(
            "Make sure Docker is installed in your system or dev environment",
        ))
        .anonymize_error()
}

#[derive(Debug, Clone, Copy, Error)]
enum DockerCmdError {
    #[error("docker-buildx is not installed")]
    BuildxNotInstalled,
    #[error("docker is not installed")]
    NotInstalled,
}
