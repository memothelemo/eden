use eden_scheduler::prelude::*;
use eden_scheduler::{JobRunner, Schedule};
use eden_utils::error::ResultExt;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "serde")]
pub struct CleanupUsers;

impl Job for CleanupUsers {
    type State = ();

    fn kind() -> &'static str
    where
        Self: Sized,
    {
        "cleanup-users"
    }

    fn schedule() -> JobSchedule
    where
        Self: Sized,
    {
        JobSchedule::interval(TimeDelta::seconds(5))
    }

    fn run(&self, _state: Self::State) -> BoxFuture<'_, eden_utils::Result<JobResult>> {
        Box::pin(async {
            panic!("Oops!");
            Ok(JobResult::Completed)
        })
    }
}

#[allow(clippy::unnecessary_wraps, clippy::unwrap_used)]
async fn bootstrap() -> eden_utils::Result<()> {
    let db_url = eden_utils::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .connect_with(PgConnectOptions::from_str(&db_url).anonymize_error()?)
        .await
        .anonymize_error()?;

    let runner = JobRunner::builder()
        .concurrency(15)
        .build(pool.clone(), ())
        .register_job::<CleanupUsers>();

    let deleted_jobs = runner.clear_all().await?;
    println!("deleted {deleted_jobs} jobs");

    runner.schedule(CleanupUsers, Schedule::now()).await?;

    runner.process_queued_jobs().await?;
    runner.queue_failed_jobs().await?;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn main() {
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(bootstrap());

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
