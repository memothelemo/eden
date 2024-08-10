use crate::docker::DockerCmdError;
use crate::{DONE_STYLE, ERROR_STYLE};

use clap::Parser;
use eden_utils::error::{exts::AnonymizedResultExt, tags::Suggestion};
use eden_utils::{build, Error, Result};
use log::{debug, info, log_enabled};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
pub struct BuildArgs {
    /// Builds Eden docker image with Rust build profile set to `debug`.
    ///
    /// It means this task will build the Eden docker image and compile Eden using
    /// `cargo build` instead of `cargo build --release` which is the default behavior
    /// if `--debug` flag` is not set.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    debug: bool,

    /// Custom image tag to create Eden docker image.
    ///
    /// When setting Docker image tag, it is suggested to use your username and any
    /// image name you want together with a backlash.
    ///
    /// Example: `memothelemo/eden`
    ///
    /// It will create a new Docker image but the new image's tag is set by the Eden's
    /// `xtask` system to `dev`.
    ///
    /// Example: `memothelemo/eden:dev`
    ///
    /// You may also set this custom tag by setting the environment variable
    /// (loading with `.env` is supported) `EDEN_XTASK_DOCKER_IMAGE_NAME` to a value
    /// followed from the previous lines.
    #[arg(short, long, env = "EDEN_XTASK_DOCKER_IMAGE_NAME")]
    tag: String,
}

pub fn run(docker_path: PathBuf, args: &BuildArgs) -> Result<()> {
    let workspace_path = env!("CARGO_WORKSPACE_DIR");

    debug!("build.commit_branch = {}", build::COMMIT_BRANCH);
    debug!("build.commit_hash = {}", build::COMMIT_HASH);
    debug!("debug = {}", args.debug);
    debug!("docker.path = {}", docker_path.to_string_lossy());
    debug!("workspace.path = {workspace_path}");
    check_buildx_installation(&docker_path)?;

    if log_enabled!(log::Level::Info) {
        info!("Building Eden docker image...");
    } else {
        println!("Building Eden docker image...");
    }

    let tag = format!("{}:dev", args.tag);

    let mut child = Command::new(docker_path)
        .args(&["build", workspace_path, "-t", &tag])
        .args(&[
            "--build-arg",
            &format!("COMMIT_HASH={}", build::COMMIT_HASH),
        ])
        .args(&[
            "--build-arg",
            &format!("COMMIT_BRANCH={}", build::COMMIT_BRANCH),
        ])
        .stdout(std::io::stdout())
        .stderr(std::io::stderr())
        .spawn()
        .expect("docker command failed to start");

    let status = child
        .wait()
        .expect("cannot wait for docker command to finish building");

    if !status.success() {
        println!();
        println!("{}", ERROR_STYLE.paint(BUILD_IMAGE_ERROR));
        std::process::exit(status.code().unwrap_or(1));
    }

    if log_enabled!(log::Level::Info) {
        info!("Building Eden docker image done");
    }

    println!("{}", DONE_STYLE.paint(BUILD_IMAGE_DONE));
    println!();
    println!("You can get access to your newly built Eden docker image with \"{tag}\".");

    Ok(())
}

const BUILDX_INSTALL_SUGGESTION: Suggestion = Suggestion::new("Make sure to install docker-buildx or its equivalent package in your distribution or preferred package manager.");
const BUILDX_NOT_COMMAND_ERR: &str = "docker: 'buildx' is not a docker command.";
const BUILD_IMAGE_ERROR: &str = "Failed to build Eden docker image! Check above this error message to diagnose and look the cause of this building image error.";
const BUILD_IMAGE_DONE: &str = "Building Eden docker image is done!";

fn check_buildx_installation(exec: &Path) -> Result<()> {
    let output = Command::new(exec)
        .arg("build")
        .output()
        .expect("failed to run process");

    if output.status.success() {
        return Ok(());
    }
    // Check if `docker: 'buildx' is not a docker command.` is specified
    let stdout = str_from_output(&output.stdout);
    let stderr = str_from_output(&output.stderr);

    let no_buildx =
        stdout.contains(BUILDX_NOT_COMMAND_ERR) || stderr.contains(BUILDX_NOT_COMMAND_ERR);

    if no_buildx {
        Err(Error::unknown(DockerCmdError::BuildxNotInstalled)).attach(BUILDX_INSTALL_SUGGESTION)
    } else {
        Ok(())
    }
}

fn str_from_output(bytes: &[u8]) -> Cow<'_, str> {
    // Windows uses UTF-16 for command outputs but...
    // https://en.wikipedia.org/wiki/Unicode_in_Microsoft_Windows#UTF-8
    String::from_utf8_lossy(bytes)
}
