use eden_tasks::prelude::*;
use eden_tasks::worker::{Worker, WorkerId};
use eden_utils::error::{exts::*, tags::Suggestion};
use eden_utils::Result;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "serde")]
struct SampleTask;

#[async_trait]
impl Task for SampleTask {
    type State = ();

    fn kind() -> &'static str
    where
        Self: Sized,
    {
        "eden::sample_task"
    }

    fn trigger() -> TaskTrigger
    where
        Self: Sized,
    {
        TaskTrigger::None
    }

    async fn perform(&self, _ctx: &TaskRunContext, _state: Self::State) -> Result<TaskResult> {
        Ok(TaskResult::Completed)
    }
}

async fn bootstrap() -> Result<()> {
    let db_url = eden_utils::env::var("DATABASE_URL")?;
    let opts = PgConnectOptions::from_str(&db_url)
        .into_typed_error()
        .attach(Suggestion::new(
            "Be sure that `DATABASE_URL` contains valid Postgres connection string",
        ))?;

    let pool = PgPoolOptions::new()
        .test_before_acquire(true)
        .connect_with(opts)
        .await
        .anonymize_error_into()?;

    let worker = Worker::<()>::new(
        WorkerId::ONE,
        pool,
        &eden_tasks::Settings::builder()
            .max_running_tasks(100.try_into().unwrap())
            .queued_tasks_per_batch(1_000.try_into().unwrap())
            .build(),
        (),
    );

    let worker = worker.register_task::<SampleTask>();
    worker.start().await?;

    eden_utils::shutdown::catch_signals().await;
    worker.shutdown().await;

    Ok(())
}

fn start() -> Result<()> {
    eden::logging::init()?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .into_typed_error()
        .attach_printable("could not build tokio runtime")?
        .block_on(bootstrap())
}

#[allow(clippy::unwrap_used)]
fn main() {
    eden::logging::install_hooks();

    if let Err(error) = start() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
