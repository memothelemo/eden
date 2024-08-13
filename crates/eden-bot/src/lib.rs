#![feature(let_chains, new_uninit)]
mod context;
mod events;
mod flags;
mod interactions;
mod local_guild;
mod suggestions;
#[cfg(test)]
mod tests;

pub mod errors;
pub mod features;
pub mod shard;
pub mod tasks;
pub mod util;

pub use self::context::{Bot, BotRef};

use self::errors::{MigrateError, StartBotError};
use eden_settings::Settings;
use eden_tasks::Scheduled;
use eden_utils::{error::exts::*, shutdown::ShutdownMode, Result};
use std::time::Duration;
use std::{sync::Arc, time::Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, trace, warn};

#[tracing::instrument(skip_all, name = "start_bot")]
pub async fn start(settings: Arc<Settings>) -> Result<(), StartBotError> {
    let bot = Bot::new(settings);
    // Run migrations first before starting the bot process entirely
    perform_database_migrations(&bot)
        .await
        .change_context(StartBotError)?;

    bot.shard_manager.start_all();

    let bot_tx = bot.clone();
    let bot_handle = eden_utils::tokio::spawn("eden_bot::start_bot", async move {
        let bot = bot_tx;
        let wait_token = Arc::new(Mutex::new(()));
        let wait_guard = wait_token.lock().await;

        // monitor if local guild exists
        tokio::spawn(monitor_for_local_guild_loaded(
            bot.clone(),
            wait_token.clone(),
        ));

        let result = bot
            .shard_manager
            .wait_for_all_connected()
            .await
            .change_context(StartBotError)
            .attach_printable("failed to connect all shards");

        drop(wait_guard);
        if result.is_err() {
            eden_utils::shutdown::trigger(ShutdownMode::Graceful).await;
            return result;
        }

        // register commands
        if let Err(error) = crate::interactions::commands::register(&bot).await {
            warn!(error = %error.anonymize(), "failed to register Eden commands. scheduling to register commands later");

            let result = bot
                .queue
                .schedule(tasks::RegisterCommands, Scheduled::in_minutes(5))
                .await;

            if let Err(error) = result {
                warn!(error = %error.anonymize(), "failed to schedule to register commands for later");
            }
        }

        eden_utils::shutdown::graceful().await;
        bot.shard_manager.shutdown_all();
        bot.shard_manager
            .wait_for_all_closed(|remaining, total| {
                info!("waiting for {remaining}/{total} shard(s) to be closed");
            })
            .await;

        Ok::<_, eden_utils::Error<StartBotError>>(())
    });

    let queue = bot.queue.clone();
    let queue_handle = eden_utils::tokio::spawn("eden_bot::start_queue", async move {
        queue.start().await.change_context(StartBotError)?;
        eden_utils::shutdown::graceful().await;

        queue.shutdown().await;
        Ok::<_, eden_utils::Error<StartBotError>>(())
    });

    let result = tokio::try_join!(bot_handle, queue_handle);
    let (bot, queue) = result
        .into_typed_error()
        .change_context(StartBotError)
        .attach_printable("one of the threads got crashed")?;

    bot?;
    queue?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn perform_database_migrations(bot: &Bot) -> Result<(), MigrateError> {
    info!("performing database migrations. this may take a while...");

    let now = Instant::now();
    eden_schema::MIGRATOR
        .run(&bot.pool)
        .await
        .into_typed_error()
        .change_context(MigrateError)?;

    let elapsed = now.elapsed();
    info!(?elapsed, "successfully performed database migrations");

    Ok(())
}

#[allow(clippy::let_underscore_must_use)]
#[tracing::instrument(skip_all)]
async fn monitor_for_local_guild_loaded(bot: Bot, wait_token: Arc<Mutex<()>>) {
    // 5 times (around 1 minute and 15 seconds of trying)
    const MAX_ATTEMPTS: usize = 5;

    debug!("monitoring to check if local guild specified exists");
    let _ = wait_token.lock().await;

    // Wait for 15 seconds, the average time to be waited for local guild
    // to load or something else it can be :)
    let mut attempts = 0;
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    loop {
        trace!("monitor loop iteration started");
        tokio::select! {
            _ = bot.shard_manager.wait_for_all_closed(|_, _| {}) => {
                trace!("all shards are closed. closing monitor loop");
                break;
            }
            _ = eden_utils::shutdown::graceful() => {
                trace!("detected shutdown from Eden. closing monitor loop");
                break;
            }
            _ = interval.tick() => {
                if bot.is_local_guild_loaded() {
                    break;
                }
                debug!(%attempts, "local guild is not loaded");

                if attempts >= MAX_ATTEMPTS {
                    warn!("Eden detects that your configured local guild does not exists and it may not work as intended!\n\n{}", suggestions::NO_LOCAL_GUILD.as_str());
                    break;
                }
                attempts += 1;
            }
        }
        trace!("monitor loop iteration ended");
    }
}
