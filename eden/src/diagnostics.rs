use eden_bot::settings::LoggingStyle;
use eden_bot::Settings;
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::{Result, Suggestion};
use tracing::level_filters::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, Layer};

const DIRECTIVES_SUGGESTION: &'static str = "Read the syntax guide for filter directives at:\nhttps://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/struct.EnvFilter.html#directives";

pub fn init(settings: &Settings) -> Result<()> {
    // I don't know how it happens but it somehow fixed the issue
    // of some events not emitted through the console likely
    // because of inconsistences `log` and `tracing` crates.
    tracing_log::LogTracer::init().attach_printable("could not initialize log tracer")?;

    #[cfg(release)]
    let default_directive = LevelFilter::WARN;
    #[cfg(not(release))]
    let default_directive = LevelFilter::INFO;

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(default_directive.into())
        .parse(settings.logging().targets())
        .anonymize_error()
        .attach_printable("could not parse log targets")
        .attach(Suggestion::new(DIRECTIVES_SUGGESTION))?;

    let log_layer = match settings.logging().style() {
        LoggingStyle::Compact => tracing_subscriber::fmt::layer()
            .compact()
            .without_time()
            .boxed(),
        LoggingStyle::Pretty => tracing_subscriber::fmt::layer()
            .pretty()
            .without_time()
            .boxed(),
        LoggingStyle::JSON => tracing_subscriber::fmt::layer()
            .json()
            .without_time()
            .boxed(),
    }
    .with_filter(env_filter);

    let subscriber = tracing_subscriber::Registry::default()
        .with(log_layer)
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .attach_printable("unable to setup tracing")?;

    Ok(())
}
