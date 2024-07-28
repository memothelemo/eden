use eden_bot::Settings;
use eden_tasks::Scheduled;

mod diagnostics;

#[allow(clippy::unnecessary_wraps, clippy::unwrap_used, clippy::unused_async)]
async fn bootstrap(settings: Settings) -> eden_utils::Result<()> {
    let bot = eden_bot::Bot::new(settings);
    bot.queue.clear_all().await?;
    bot.queue.start().await?;

    bot.queue
        .schedule(eden_bot::tasks::TestTask, Scheduled::in_seconds(3))
        .await?;

    eden_utils::shutdown::catch_signals().await;
    bot.queue.shutdown().await;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn start() -> eden_utils::Result<()> {
    let settings = Settings::from_env()?;

    self::diagnostics::init(&settings)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(bootstrap(settings))
}

fn main() {
    eden_utils::Suggestion::install_hooks();
    eden_utils::Error::init();

    if let Err(error) = start() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
