use eden_settings::Settings;
use eden_utils::{
    error::exts::{AnonymizeErrorInto, AnonymizedResultExt},
    Result,
};

const EXAMPLE_SETTINGS_FILE: &str =
    concat!(env!("CARGO_WORKSPACE_DIR"), "config/eden.example.toml");

pub fn run() -> Result<()> {
    let contents = Settings::generate_docs();
    std::fs::write(EXAMPLE_SETTINGS_FILE, contents)
        .anonymize_error_into()
        .attach_printable_lazy(|| format!("could not write file for {EXAMPLE_SETTINGS_FILE}"))?;

    println!("Generated settings file at: {EXAMPLE_SETTINGS_FILE}");
    Ok(())
}
