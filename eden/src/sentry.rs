use eden_settings::Settings;
use sentry::{ClientInitGuard, ClientOptions, TransactionContext};
use std::sync::Arc;
use tracing::info;

pub struct EdenSentryGuard {
    guard: Option<ClientInitGuard>,
}

impl Drop for EdenSentryGuard {
    fn drop(&mut self) {
        info!("waiting for all pending events to be sent to Sentry");
        let guard = self.guard.take();
        drop(guard);
        info!("done sending pending events to Sentry");
    }
}

/// Initializes the Sentry SDK from [settings](eden_settings::Settings).
#[allow(clippy::unwrap_used)]
pub fn init(settings: &Settings) -> Option<EdenSentryGuard> {
    let Some(settings) = settings.sentry.as_ref() else {
        return None;
    };

    info!(
        sentry.environment = ?settings.environment,
        sentry.traces_sample_rate = %settings.traces_sample_rate,
        "sentry integration is enabled"
    );

    let traces_sample_rate = settings.traces_sample_rate;
    let traces_sampler = move |ctx: &TransactionContext| {
        // Copied from: https://github.com/rust-lang/crates.io/blob/679a96270fe9209b31c071c55432e8e2aa1d013e/src/sentry/mod.rs#L23-L25
        // Licensed under MIT/Apache-2.0
        if let Some(sampled) = ctx.sampled() {
            return if sampled { 1.0 } else { 0.0 };
        }
        traces_sample_rate
    };

    let opts = ClientOptions {
        auto_session_tracking: true,
        dsn: Some(settings.dsn.as_ref().clone()),
        environment: Some(settings.environment.clone().into()),
        release: sentry::release_name!(),
        session_mode: sentry::SessionMode::Application,
        traces_sampler: Some(Arc::new(traces_sampler)),
        ..Default::default()
    };

    Some(EdenSentryGuard {
        guard: Some(sentry::init(opts)),
    })
}
