use eden_settings::Settings;
use eden_tasks::QueueWorker;
use sqlx::postgres::PgPoolOptions;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Weak;
use std::sync::{atomic::AtomicU64, Arc};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_http::client::InteractionClient;
use twilight_model::id::{marker::ApplicationMarker, Id};

use crate::interactions::InMemoryCommandState;
use crate::shard::ShardManager;

// involves database functionality for Bot struct.
mod database;
// useful functions that will make my life easier
mod util;

pub struct BotInner {
    pub cache: Arc<InMemoryCache>,
    pub http: Arc<twilight_http::Client>,
    pub queue: BotQueue,
    pub command_state: Arc<InMemoryCommandState>,
    pub pool: sqlx::PgPool,
    pub shard_manager: Arc<ShardManager>,
    pub settings: Arc<Settings>,

    // Since application IDs are just u64 values, we can retain it
    // as long as it is a valid Twilight application ID.
    application_id: AtomicU64,
    is_local_guild_loaded: AtomicBool,
}

impl Bot {
    #[allow(clippy::unwrap_used)]
    pub fn new(settings: Arc<Settings>) -> Self {
        let mut http = twilight_http::Client::builder()
            .timeout(settings.bot.http.timeout)
            .token(settings.bot.token.expose().into());

        if let Some(proxy) = settings.bot.http.proxy.as_ref() {
            http = http.proxy(proxy.as_str().into(), settings.bot.http.proxy_use_http);
        }

        let cache = InMemoryCache::builder()
            .resource_types(if settings.bot.http.use_cache {
                crate::flags::CACHE_RESOURCE_TYPES
            } else {
                ResourceType::empty()
            })
            .build();

        let http = Arc::new(http.build());
        let cache = Arc::new(cache);

        let connect_options = settings.database.as_postgres_connect_options();
        let statement_timeout = settings.database.query_timeout;

        let pool = PgPoolOptions::new()
            .idle_timeout(settings.database.idle_timeout)
            .acquire_timeout(settings.database.connect_timeout)
            .max_connections(settings.database.max_connections)
            .min_connections(settings.database.min_connections)
            .test_before_acquire(true)
            .after_connect(move |conn, _metadata| {
                Box::pin(async move {
                    sqlx::query(r"SET application_name = 'eden'")
                        .execute(&mut *conn)
                        .await?;

                    let timeout = statement_timeout.as_millis();
                    sqlx::query(&format!("SET statement_timeout = {timeout}"))
                        .execute(conn)
                        .await?;

                    Ok(())
                })
            })
            .connect_lazy_with(connect_options);

        let inner = Arc::<BotInner>::new_cyclic(move |bot_weak| {
            let bot_weak = BotRef(bot_weak.clone());
            let queue = crate::tasks::register_all_tasks(QueueWorker::new(
                settings.worker.id,
                pool.clone(),
                &settings.worker,
                bot_weak.clone(),
            ));
            let shard_manager = ShardManager::new(bot_weak.clone(), settings.clone());
            BotInner {
                // no application id of 0 in twilight-model will accept this
                application_id: AtomicU64::new(0),
                cache,
                is_local_guild_loaded: AtomicBool::new(false),
                http,
                command_state: InMemoryCommandState::new(bot_weak.clone()),
                queue,
                shard_manager,
                settings,
                pool,
            }
        });

        Self(inner)
    }

    /// Gets the resolved application ID if it is loaded.
    #[must_use]
    pub fn application_id(&self) -> Id<ApplicationMarker> {
        if let Some(id) = self.checked_application_id() {
            id
        } else {
            panic!("tried to access bot's application id while the bot is not ready");
        }
    }

    /// Gets the resolved application ID if it is loaded.
    #[must_use]
    pub fn checked_application_id(&self) -> Option<Id<ApplicationMarker>> {
        let value = self.0.application_id.load(Ordering::Relaxed);
        Id::<ApplicationMarker>::new_checked(value)
    }

    #[must_use]
    pub fn is_cache_enabled(&self) -> bool {
        self.0.settings.bot.http.use_cache
    }

    #[must_use]
    pub fn is_local_guild_loaded(&self) -> bool {
        self.is_local_guild_loaded.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn interaction(&self) -> InteractionClient<'_> {
        let Some(application_id) = self.checked_application_id() else {
            panic!("tried to call bot.interaction while the bot is not ready");
        };
        self.0.http.interaction(application_id)
    }

    pub(crate) fn on_local_guild_loaded(&self) {
        self.is_local_guild_loaded.store(true, Ordering::Relaxed);
    }

    pub(crate) fn override_application_id(&self, id: Id<ApplicationMarker>) {
        self.0.application_id.store(id.get(), Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct Bot(Arc<BotInner>);

impl Deref for Bot {
    type Target = BotInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Bot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bot")
            .field("application_id", &self.application_id())
            .field("http", &self.http)
            .field("pool", &self.pool)
            .field("queue", &self.queue)
            .field("settings", &self.settings)
            .finish()
    }
}

/// A weak reference version of the actual [`Bot`] object.
///
/// This type exists in the first place is to avoid memory cyclic
/// reference issues with Eden while trying to use the `unsafe`
/// method to get around cyclic references.
#[derive(Clone)]
pub struct BotRef(Weak<BotInner>);

impl std::fmt::Debug for BotRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bot").finish_non_exhaustive()
    }
}

impl BotRef {
    /// Converts a weak reference of [`Bot`] object into the actual
    /// [`Bot`] object itself.
    #[allow(clippy::expect_used)]
    pub fn get(&self) -> Bot {
        let inner = self
            .0
            .upgrade()
            .expect("unexpected drop from Arc<BotInner>");

        Bot(inner)
    }
}

pub(crate) type BotQueue = QueueWorker<BotRef>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[should_panic]
    async fn should_crash_in_interaction_fn_if_no_application_id() {
        let settings = Arc::new(crate::tests::generate_fake_settings());
        let bot = Bot::new(settings.clone());
        let _client = bot.interaction();
    }

    #[tokio::test]
    async fn test_is_cache_enabled() {
        let mut settings = crate::tests::generate_fake_settings();
        settings.bot.http.use_cache = false;

        let settings = Arc::new(settings);
        let bot = Bot::new(settings.clone());
        assert_eq!(bot.is_cache_enabled(), false);
    }

    #[tokio::test]
    async fn test_override_application_id() {
        let settings = crate::tests::generate_fake_settings();
        let bot = Bot::new(Arc::new(settings));
        assert_eq!(bot.checked_application_id(), None);

        let new_id = Id::new(273534239310479360);
        bot.override_application_id(new_id);
        assert_eq!(bot.checked_application_id(), Some(new_id));
    }
}
