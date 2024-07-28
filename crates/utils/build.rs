use anyhow::Result;

fn emit_git_variables() -> Result<()> {
    let mut builder = vergen_git2::Git2Builder::default();
    builder.branch(true);
    builder.commit_timestamp(true);
    builder.sha(true);

    let git2 = builder.build()?;
    vergen_git2::Emitter::default()
        .fail_on_error()
        .add_instructions(&git2)?
        .emit()
}

fn main() {
    if let Ok(value) = std::env::var("PROFILE") {
        println!("cargo:rustc-env=BUILD_PROFILE={value}");
    } else {
        println!("cargo:error=failed to get build profile");
    }

    if let Err(error) = emit_git_variables() {
        println!("cargo:warning=cannot load git repository of Eden: {error}");
    }
}
