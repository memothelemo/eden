use eden_settings::{LoggingStyle, Settings};
use eden_utils::build;
use eden_utils::error::tags::Suggestion;
use eden_utils::{error::exts::*, Result};
use sentry::integrations::tracing::EventFilter;
use tracing::level_filters::LevelFilter;
use tracing::{Level, Metadata};
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, Layer};

const DIRECTIVES_SUGGESTION: &'static str = "Read the syntax guide for filter directives at:\nhttps://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/struct.EnvFilter.html#directives";

pub fn init(settings: &Settings) -> Result<()> {
    // I don't know how it happens but it somehow fixed the issue
    // of some events not emitted through the console likely
    // because of inconsistences `log` and `tracing` crates.
    tracing_log::LogTracer::init()
        .into_typed_error()
        .attach_printable("could not initialize log tracer")?;

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(if build::PROFILE == "release" {
            LevelFilter::WARN.into()
        } else {
            LevelFilter::INFO.into()
        })
        .parse(&settings.logging.targets)
        .into_typed_error()
        .attach_printable("could not parse log targets")
        .attach(Suggestion::new(DIRECTIVES_SUGGESTION))?;

    let sentry_filter = if let Some(sentry) = settings.sentry.as_ref() {
        let filter = tracing_subscriber::EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .parse(&sentry.targets)
            .into_typed_error()
            .attach_printable("could not parse log targets for `sentry.targets`")
            .attach(Suggestion::new(DIRECTIVES_SUGGESTION))?;

        Some(filter)
    } else {
        None
    };

    let log_layer = match settings.logging.style {
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

    let sentry_layer = sentry::integrations::tracing::layer()
        .event_filter(event_filter)
        .with_filter(sentry_filter);

    let subscriber = tracing_subscriber::Registry::default()
        .with(log_layer)
        .with(sentry_layer)
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .into_typed_error()
        .attach_printable("unable to setup tracing")?;

    Ok(())
}

fn event_filter(metadata: &Metadata<'_>) -> EventFilter {
    let has_error = metadata.fields().iter().any(|v| v.name() == "error");
    match metadata.level() {
        &Level::ERROR => EventFilter::Exception,
        &Level::WARN if has_error => EventFilter::Exception,
        &Level::WARN | &Level::INFO => EventFilter::Breadcrumb,
        &Level::DEBUG | &Level::TRACE => EventFilter::Ignore,
    }
}

/// Installs error from across all crates of Eden project.
pub fn install_hooks() {
    use eden_utils::Error;

    Error::init();
    eden_bot::errors::tags::install_hook();
    eden_tasks::error::tags::install_hook();
}
