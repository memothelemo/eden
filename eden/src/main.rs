use eden_settings::Settings;
use eden_utils::error::exts::*;
use eden_utils::Result;
use std::sync::Arc;

async fn bootstrap(settings: Settings) -> Result<()> {
    let result = tokio::try_join!(eden_bot::start(Arc::new(settings)), async {
        eden_utils::shutdown::catch_signals().await;
        Ok(())
    });

    result.map(|(_, bot)| bot).anonymize_error()
}

fn start() -> Result<()> {
    let settings = Settings::from_env()?;
    eden::logging::init(&settings)?;
    eden::print_launch(&settings);

    let _sentry = eden::sentry::init(&settings);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(settings.threads)
        .build()
        .into_typed_error()
        .attach_printable("could not build tokio runtime")?
        .block_on(bootstrap(settings))
}

#[allow(clippy::unwrap_used)]
fn main() {
    eden::logging::install_hooks();

    if let Err(error) = start() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
