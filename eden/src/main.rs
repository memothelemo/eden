// use eden_scheduler::prelude::*;
// use eden_scheduler::scheduler::Scheduled;
use eden_utils::error::ResultExt;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;

// #[derive(Debug, Deserialize, Serialize)]
// #[serde(crate = "serde")]
// pub struct CleanupUsers;

// impl Task for CleanupUsers {
//     type State = ();

//     fn kind() -> &'static str
//     where
//         Self: Sized,
//     {
//         "cleanup-users"
//     }

//     fn schedule() -> TaskSchedule
//     where
//         Self: Sized,
//     {
//         TaskSchedule::None
//     }

//     fn perform(&self, _state: Self::State) -> BoxFuture<'_, eden_utils::Result<TaskResult>> {
//         Box::pin(async { Ok(TaskResult::Completed) })
//     }
// }

#[allow(clippy::unnecessary_wraps, clippy::unwrap_used)]
async fn bootstrap() -> eden_utils::Result<()> {
    let db_url = eden_utils::env::var("DATABASE_URL")?;
    let _pool = PgPoolOptions::new()
        .connect_with(PgConnectOptions::from_str(&db_url).anonymize_error()?)
        .await
        .anonymize_error()?;

    // let scheduler = eden_scheduler::TaskScheduler::builder()
    //     .concurrency(25)
    //     .build(pool.clone(), ())
    //     .register_task::<CleanupUsers>();

    // let deleted_jobs = scheduler.clear_all().await?;
    // println!("deleted {deleted_jobs} jobs");

    // scheduler.schedule(CleanupUsers, Scheduled::now()).await?;
    // scheduler.queue(CleanupUsers).await?;

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
