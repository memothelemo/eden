use eden_utils::error::{exts::*, tags::Suggestion};
use eden_utils::sql::SqlErrorExt;
use eden_utils::{build, Result};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;
use std::time::Instant;

async fn bootstrap() -> Result<()> {
    // let db_url = eden_utils::env::var("DATABASE_URL")?;
    // let opts = PgConnectOptions::from_str(&db_url)
    //     .into_typed_error()
    //     .attach(Suggestion::new(
    //         "Be sure that `DATABASE_URL` contains valid Postgres connection string",
    //     ))?;

    // let pool = PgPoolOptions::new()
    //     .test_before_acquire(true)
    //     .connect_with(opts)
    //     .await
    //     .anonymize_error_into()?;

    // let result = sqlx::query("(")
    //     .execute(&mut *pool.acquire().await.anonymize_error_into()?)
    //     .await
    //     .anonymize_error_into();

    // if result.is_pool_error() {
    //     println!("Oops!");
    // }

    // result?;

    Ok(())
}

#[allow(clippy::unwrap_used)]
fn main() {
    eden_utils::Error::init();

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
