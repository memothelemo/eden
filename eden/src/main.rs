use eden_tasks::prelude::*;
use eden_utils::error::ResultExt;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, Layer};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "serde")]
pub struct CleanupUsers;

#[async_trait]
impl Task for CleanupUsers {
    type State = ();

    fn task_type() -> &'static str
    where
        Self: Sized,
    {
        "cleanup-users"
    }

    fn schedule() -> TaskSchedule
    where
        Self: Sized,
    {
        TaskSchedule::interval(TimeDelta::seconds(10))
    }

    #[allow(clippy::unwrap_used)]
    async fn perform(
        &self,
        _info: &TaskPerformInfo,
        _state: Self::State,
    ) -> eden_utils::Result<TaskResult> {
        Ok(TaskResult::Completed)
    }
}

#[allow(clippy::unnecessary_wraps, clippy::unwrap_used)]
async fn bootstrap() -> eden_utils::Result<()> {
    let db_url = eden_utils::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .connect_with(PgConnectOptions::from_str(&db_url).anonymize_error()?)
        .await
        .anonymize_error()?;

    let queue = eden_tasks::Queue::builder()
        .concurrency(25)
        .periodic_poll_interval(TimeDelta::seconds(1))
        .build(pool.clone(), ())
        .register_task::<CleanupUsers>();

    queue.clear_all().await?;
    queue.start().await?;

    eden_utils::shutdown_signal().await;
    queue.shutdown().await;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn start() -> eden_utils::Result<()> {
    tracing_log::LogTracer::init().attach_printable("could not initialize log tracer")?;

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse(eden_utils::env::var("RUST_LOG")?)
        .anonymize_error()?;

    let log_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter(env_filter);

    let subscriber = tracing_subscriber::Registry::default()
        .with(log_layer)
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .attach_printable("unable to setup tracing")?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(bootstrap())
}

fn main() {
    eden_utils::Error::init();

    if let Err(error) = start() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
