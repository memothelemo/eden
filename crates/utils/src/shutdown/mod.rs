mod signal;
mod state;

use self::signal::{abort_signal, shutdown_signal};
use self::state::{ShutdownFuture, STATE};

use std::sync::atomic::Ordering;

/// Attempts to perform graceful shutdown the entire process without
/// the user or the host trigger the shutdown signal.
pub fn shutdown() {
    #[cfg(not(release))]
    tracing::info!("requested shutdown. performing graceful shutdown...");
    #[cfg(release)]
    println!("Requested shutdown. Shutting down Eden instance...");
    STATE.graceful_notify.notify_waiters();
}

pub async fn catch_signals() {
    let occupied = STATE.catch_signals_guard.get().is_some();
    if occupied {
        panic!("Only one thread or future to catch shutdown signals");
    }

    // We don't need to catch signals again if it already shutted down
    if is_shutted_down() {
        return;
    }

    let signal = shutdown_signal();
    let manual_shutdown = graceful();
    tokio::select! {
        _ = signal => {
            #[cfg(not(release))]
            tracing::warn!("received shutdown signal. performing graceful shutdown...");
            #[cfg(release)]
            println!("Requested shutdown. Performing graceful shutdown...");

            STATE.graceful_notify.notify_waiters();
        },
        _ = manual_shutdown => {}
    }

    // Spawn another thread to monitor if shutdown signal is
    // triggered again. if it is then the user requested for
    // aborted shutdown
    tokio::spawn(async {
        abort_signal().await;
        #[cfg(not(release))]
        tracing::warn!("received abort signal. aborting process...");
        #[cfg(release)]
        println!("Requested shutdown again. Aborting Eden instance...");
        STATE.abort_notify.notify_waiters();
    });
}

pub fn graceful() -> ShutdownFuture {
    ShutdownFuture::graceful()
}

pub fn aborted() -> ShutdownFuture {
    ShutdownFuture::abort()
}

pub fn is_shutted_down() -> bool {
    STATE.graceful.load(Ordering::SeqCst)
}
