#![feature(let_chains, new_uninit)]
mod context;

pub use self::context::*;
pub use self::settings::Settings;
pub use self::shard::ShardContext;

pub mod error;
pub mod events;
pub mod interaction;
pub mod settings;
pub mod shard;
pub mod tasks;

use self::error::StartBotError;
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::Result;
use std::sync::Arc;
use tokio::task::JoinSet;
use twilight_gateway::{Shard, ShardId};

pub async fn start(settings: Arc<Settings>) -> Result<(), StartBotError> {
    let bot = Bot::new(settings);
    bot.test_db_pool().await.transform_context(StartBotError)?;

    let shards = create_shards(&bot)
        .await
        .attach_printable("could not create shards")?;

    // Resolve bot's application id
    if bot.settings.bot.application_id.is_none() {
        tracing::debug!("`bot.application_id` is missing. fetching bot information...");
        update_bot_info(&bot)
            .await
            .attach_printable(format!("could not fetch bot information"))?;
    }

    // To avoid sending multiple register command requests to Discord
    tracing::debug!("clearing queued register command tasks");
    bot.queue
        .clear_all_with::<tasks::RegisterCommands>()
        .await
        .change_context(StartBotError)?;

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
    let gateway_config = Config::builder(token, events::INTENTS)
        .event_types(events::FILTERED_EVENT_TYPES)
        .build();

    let shards = match bot.settings.bot.sharding {
        Sharding::Single { id, total } => {
            let id = id.clamp(0, total.get());
            let id = ShardId::new(id, total.get());
            vec![Shard::with_config(id, gateway_config)]
        }
        Sharding::Range {
            start, end, total, ..
        } => {
            let start = start.clamp(0, total.get() - 1);
            let end = end.get().clamp(start, total.get() - 1);
            let start = start.clamp(0, end);
            create_range(start..end, total.get(), gateway_config, |_, builder| {
                builder.build()
            })
            .collect::<Vec<_>>()
        }
    };

    Ok(shards)
}

async fn init_all_shards(bot: Bot, shards: Vec<Shard>) -> Result<(), StartBotError> {
    #[cfg(not(release))]
    tracing::info!("starting bot with {} shards", shards.len());

    #[cfg(release)]
    println!("Starting bot with {} shards", shards.len());

    // The release version notice that caching is enabled is in eden::print_launch
    #[cfg(not(release))]
    tracing::debug!(
        "caching is {}",
        if bot.settings.bot.http.use_cache {
            "enabled"
        } else {
            "disabled"
        }
    );

    let mut running_shards = JoinSet::new();
    let (observer_tx, observer_rx) = tokio::sync::mpsc::unbounded_channel();

    let total_shards = shards.len();
    let observe_shards_handle = tokio::spawn(shard::observe_shards(
        bot.clone(),
        shards.len(),
        observer_rx,
    ));

    for shard in shards {
        running_shards.spawn(self::shard::main(shard, bot.clone(), observer_tx.clone()));
    }

    eden_utils::shutdown::graceful().await;
    tracing::info!("closing bot service");

    loop {
        let closed_shards = total_shards - running_shards.len();
        #[cfg(not(release))]
        tracing::info!("waiting for {closed_shards}/{total_shards} shard(s) to finish");
        #[cfg(release)]
        println!("Waiting for {closed_shards}/{total_shards} shard(s) to finish");

        if running_shards.join_next().await.is_none() {
            break;
        }
    }

    observe_shards_handle
        .await
        .change_context(StartBotError)
        .attach_printable("shard observer thread got crashed")?;

    tracing::info!("all shard(s) are closed");
    Ok(())
}

async fn update_bot_info(bot: &Bot) -> Result<(), StartBotError> {
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
