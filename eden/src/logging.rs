use eden_utils::error::tags::Suggestion;
use eden_utils::{error::exts::*, Result};
use tracing::level_filters::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, Layer};

const DIRECTIVES_SUGGESTION: &'static str = "Read the syntax guide for filter directives at:\nhttps://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/struct.EnvFilter.html#directives";

pub fn init() -> Result<()> {
    // I don't know how it happens but it somehow fixed the issue
    // of some events not emitted through the console likely
    // because of inconsistences `log` and `tracing` crates.
    tracing_log::LogTracer::init()
        .into_typed_error()
        .attach_printable("could not initialize log tracer")?;

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse(eden_utils::env::var("RUST_LOG")?)
        .into_typed_error()
        .attach_printable("could not parse log targets")
        .attach(Suggestion::new(DIRECTIVES_SUGGESTION))?;

    let log_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .without_time()
        .boxed()
        .with_filter(env_filter);

    let subscriber = tracing_subscriber::Registry::default()
        .with(log_layer)
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .into_typed_error()
        .attach_printable("unable to setup tracing")?;

    Ok(())
}

/// Installs error from across all crates of Eden project.
pub fn install_hooks() {
    use eden_utils::Error;

    Error::init();
    eden_tasks::error::tags::install_hook();
}
