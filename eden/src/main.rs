#![feature(result_flattening)]
use eden_bot::error::StartBotError;
use eden_bot::Settings;
use eden_utils::error::ResultExt;

#[allow(clippy::unnecessary_wraps, clippy::unwrap_used, clippy::unused_async)]
async fn bootstrap(settings: Settings) -> eden_utils::Result<()> {
    let settings = std::sync::Arc::new(settings);
    let (_, bot) = tokio::join!(
        eden_utils::shutdown::catch_signals(),
        tokio::spawn(eden_bot::start(settings))
    );

    bot.change_context(StartBotError)
        .attach_printable("thread got crashed")
        .flatten()?;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn start() -> eden_utils::Result<()> {
    println!("{}", Settings::generate_docs());
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
