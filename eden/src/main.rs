use eden_bot::shard::ShardManager;
use eden_bot::Bot;
use eden_settings::Settings;
use eden_utils::error::exts::*;
use eden_utils::Result;
use std::sync::Arc;

async fn bootstrap(settings: Settings) -> Result<()> {
    let bot = Bot::new(Arc::new(settings));
    let shard_manager = ShardManager::new(bot.clone());
    shard_manager.start_all();
    shard_manager.wait_for_all_connected().await;

    eden_utils::shutdown::catch_signals().await;
    shard_manager.shutdown_all();
    shard_manager.wait_for_all_closed().await;

    Ok(())
}

fn start() -> Result<()> {
    let settings = Settings::from_env()?;

    eden::print_launch(&settings);
    eden::logging::init(&settings)?;

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
