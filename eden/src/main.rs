#![feature(result_flattening)]
use eden_bot::error::StartBotError;
use eden_bot::Settings;
use eden_utils::error::ResultExt;

#[allow(clippy::unnecessary_wraps, clippy::unwrap_used, clippy::unused_async)]
async fn bootstrap(settings: Settings) -> eden_utils::Result<()> {
    let settings = std::sync::Arc::new(settings);
    let result = tokio::try_join!(eden_bot::start(settings), async {
        eden_utils::shutdown::catch_signals().await;
        Ok(())
    });

    result
        .map(|(_, bot)| bot)
        .change_context(StartBotError)
        .attach_printable("failed to process threads")?;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn start() -> eden_utils::Result<()> {
    let settings = Settings::from_env()?;

    #[cfg(release)]
    eden::print_launch(&settings);
    eden::diagnostics::init(&settings)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(bootstrap(settings))
}

fn main() {
    eden::install_prerequisite_hooks();

    if let Err(error) = start() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
