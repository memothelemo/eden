#![feature(new_uninit)]
mod context;

pub use self::context::*;
pub use self::settings::Settings;

pub mod error;
pub mod events;
pub mod settings;
pub mod shard;
pub mod tasks;

use self::error::StartBotError;
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::Result;
use std::sync::Arc;
use tokio::task::JoinSet;
use twilight_gateway::{Intents, Shard, ShardId};

const INTENTS: Intents = Intents::GUILDS;

pub async fn start(settings: Arc<Settings>) -> Result<(), StartBotError> {
    let bot = Bot::new(settings);
    let shards = create_shards(&bot)
        .await
        .attach_printable("could not create shards")?;

    // Resolve bot's application id
    if bot.application_id.get().is_none() {
        tracing::debug!("`bot.application_id` is missing. fetching bot information...");
        update_bot_info(&bot)
            .await
            .attach_printable(format!("could not fetch bot information"))?;
    }

    bot.queue.start().await.change_context(StartBotError)?;

    let queue_tx = bot.queue.clone();
    let queue_shutdown_handle = tokio::spawn(async move {
        eden_utils::shutdown::graceful().await;
        queue_tx.shutdown().await;
    });

    let (shards, queue) = tokio::join!(init_all_shards(bot, shards), queue_shutdown_handle);
    shards
        .anonymize_error()
        .transform_context(StartBotError)
        .attach_printable("shards thread crashed")?;

    queue
        .anonymize_error()
        .transform_context(StartBotError)
        .attach_printable("queue shutdown thread crashed")?;

    Ok(())
}

// TODO: Find a way on how to configure multiple shards in a single instance
async fn create_shards(bot: &Bot) -> Result<Vec<Shard>, StartBotError> {
    use self::settings::Sharding;
    use twilight_gateway::stream::create_range;
    use twilight_gateway::Config;

    let token = bot.settings.bot.token.as_str().to_string();
    let shards = match bot.settings.bot.sharding {
        Sharding::Single { id, total } => {
            let id = id.clamp(0, total.get());
            vec![Shard::new(ShardId::new(id, total.get()), token, INTENTS)]
        }
        Sharding::Range {
            start, end, total, ..
        } => {
            let shared_config = Config::builder(token, INTENTS).build();
            let start = start.clamp(0, total.get() - 1);
            let end = end.get().clamp(start, total.get() - 1);
            let start = start.clamp(0, end);
            create_range(start..end, total.get(), shared_config, |_, builder| {
                builder.build()
            })
            .collect::<Vec<_>>()
        }
    };

    Ok(shards)
}

async fn init_all_shards(bot: Bot, shards: Vec<Shard>) -> Result<(), StartBotError> {
    tracing::info!("starting bot with {} shards", shards.len());

    let mut running_shards = JoinSet::new();
    let (observer_tx, observer_rx) = tokio::sync::mpsc::unbounded_channel();
    let total_shards = shards.len();

    let shards_observer_handle = tokio::spawn(shard::observe_shards(shards.len(), observer_rx));
    for shard in shards {
        running_shards.spawn(self::shard::main(shard, bot.clone(), observer_tx.clone()));
    }

    eden_utils::shutdown::graceful().await;
    tracing::info!("closing bot service");

    loop {
        let closed_shards = total_shards - running_shards.len();
        tracing::info!("waiting for {closed_shards}/{total_shards} shard(s) to finish");

        if running_shards.join_next().await.is_none() {
            break;
        }
    }

    shards_observer_handle
        .await
        .change_context(StartBotError)
        .attach_printable("shard observer thread got crashed")?;

    tracing::info!("all shard(s) are closed");
    Ok(())
}

async fn update_bot_info(bot: &Bot) -> Result<(), StartBotError> {
    // TODO: twilight-http does not cancel the future if response fails. try implementing timeout
    let response = bot
        .http
        .current_user_application()
        .await
        .map_err(eden_utils::Error::unknown)
        .transform_context(StartBotError)?;

    if !response.status().is_success() {
        let text = response
            .text()
            .await
            .attach_printable("could not get error information")
            .change_context(StartBotError)?;

        return Err(eden_utils::Error::context(
            eden_utils::ErrorCategory::Unknown,
            StartBotError,
        ))
        .attach_printable(format!("{}", text));
    }

    let application = response
        .model()
        .await
        .map_err(eden_utils::Error::unknown)
        .transform_context(StartBotError)?;

    tracing::debug!("logged in as {:?} ({})", application.name, application.id);
    if bot.application_id.set(application.id).is_err() {
        tracing::warn!("unexpected bot.application_id to be defined");
    }

    Ok(())
}
