use eden_tasks::Queue;
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::Result;
use once_cell::sync::OnceCell;
use sqlx::postgres::PgPoolOptions;
use std::{fmt::Debug, mem::MaybeUninit, ops::Deref, sync::Arc};
use twilight_http::client::InteractionClient;
use twilight_http::Client as HttpClient;

use crate::Settings;

/// It holds the main components of the application such as
/// settings, database connection pool and Eden's own task
/// queueing system.
#[derive(Clone)]
pub struct Bot(Arc<BotInner>);

impl Bot {
    #[allow(clippy::unwrap_used)]
    pub fn new(settings: Arc<Settings>) -> Self {
        let mut http = HttpClient::builder()
            .timeout(settings.bot.http.timeout)
            .token(settings.bot.token.as_str().into());

        if let Some(proxy) = settings.bot.http.proxy.as_ref() {
            http = http.proxy(proxy.as_str().into(), settings.bot.http.proxy_use_http);
        }

        let application_id = OnceCell::new();
        if let Some(value) = settings.bot.application_id {
            match application_id.set(value) {
                Ok(..) => {}
                Err(..) => panic!("unexpected application_id is full"),
            }
        }

        let http = Arc::new(http.build());
        let pool = PgPoolOptions::new()
            .idle_timeout(settings.database.idle_timeout)
            .acquire_timeout(settings.database.connect_timeout)
            .max_connections(settings.database.max_connections)
            .min_connections(settings.database.min_connections)
            .test_before_acquire(true)
            .connect_lazy_with(settings.database.connect_options());

        let queue = Queue::builder()
            .concurrency(settings.queue.max_running_tasks)
            .max_attempts(settings.queue.max_task_retries)
            .stalled_tasks_threshold(settings.queue.stalled_tasks_threshold)
            .periodic_poll_interval(settings.queue.polling.periodic)
            .queue_poll_interval(settings.queue.polling.queue);

        // SAFETY: The bot object (state for the queue object) will not be
        //         used when Queue::builder().build() is called and registered
        //         all required tasks with 'crate::tasks::register_all_tasks'.
        //
        //         The inner value of the Bot object will be eventually replaced
        //         since we want to have queue obect stored with `Bot` as its state
        //         at the same time keep the queue inside the Bot object.
        let inner_uninit = Arc::new_zeroed();
        let bot = Bot(unsafe { inner_uninit.clone().assume_init() });

        let queue = queue.build(pool.clone(), bot.clone());
        let queue = crate::tasks::register_all_tasks(queue);
        unsafe {
            let inner = &mut *(Arc::as_ptr(&inner_uninit) as *mut MaybeUninit<BotInner>);
            inner.write(BotInner {
                application_id,
                http,
                pool,
                queue,
                settings,
            });
        }

        bot
    }
}

impl Bot {
    pub fn interaction(&self) -> InteractionClient<'_> {
        let Some(application_id) = self.application_id.get().copied() else {
            panic!("tried to call bot.interaction while the bot is not ready");
        };
        self.0.http.interaction(application_id)
    }

    pub async fn test_db_pool(&self) -> Result<()> {
        tracing::debug!("testing database pool...");

        match self.0.pool.acquire().await {
            Ok(..) => Ok(()),
            Err(error) if error.as_database_error().is_none() => Err(error)
                .anonymize_error()
                .attach_printable("database is unhealthy"),
            Err(error) => Err(error).anonymize_error(),
        }
    }
}

impl Deref for Bot {
    type Target = BotInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Bot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bot")
            .field("application_id", &DebugAppId(&self.0.application_id))
            .field("http", &NonExhaustiveFields("Client"))
            .field("pool", &self.0.pool)
            .field("queue", &self.0.queue)
            .field("settings", &self.0.settings)
            .finish()
    }
}

// dealing with rust's requirement of making BotInner public
// to implement Deref for Bot object.
mod private {
    use super::*;
    use twilight_model::id::{marker::ApplicationMarker, Id};

    #[derive(Debug)]
    pub struct BotInner {
        pub application_id: OnceCell<Id<ApplicationMarker>>,
        pub http: Arc<HttpClient>,
        pub pool: sqlx::PgPool,
        pub queue: Queue<Bot>,
        pub settings: Arc<Settings>,
    }

    pub struct DebugAppId<'a>(pub &'a OnceCell<Id<ApplicationMarker>>);

    impl<'a> Debug for DebugAppId<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(id) = self.0.get() {
                write!(f, "{}", id.get())
            } else {
                write!(f, "<none>")
            }
        }
    }

    pub struct NonExhaustiveFields(pub &'static str);

    impl Debug for NonExhaustiveFields {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(&self.0).finish_non_exhaustive()
        }
    }
}
use self::private::*;
