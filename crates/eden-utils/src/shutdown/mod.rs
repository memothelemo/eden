use std::sync::{LazyLock, OnceLock};
use tokio::sync::{Mutex, Notify};
use tracing::warn;

mod futures;
use self::futures::WaitForShutdownFuture;

pub async fn catch_signals() {
    let occupied = STATE.catch_signals_guard.get().is_some();
    if occupied {
        panic!("Only one thread or future to catch shutdown signals");
    }

    // We don't need to catch signals again if it already shutted down
    if get_shutdown_mode().await.is_some() {
        return;
    }

    // unless the user actually requested for an abort shutdown
    let signal = signals::graceful();
    let triggered = graceful();
    let aborted = aborted();

    tokio::select! {
        _ = signal => {
            warn!("received shutdown signal. performing graceful shutdown...");

            *STATE.mode.lock().await = Some(ShutdownMode::Graceful);
            STATE.mode_changed.notify_waiters();
        },
        _ = triggered => {}
        _ = aborted => {}
    }

    // Spawn another thread to monitor if shutdown signal is
    // triggered again. if it is then the user requested for
    // abort shutdown.
    if matches!(&get_shutdown_mode().await, Some(ShutdownMode::Abort)) {
        return;
    }

    tokio::spawn(async {
        signals::abort().await;

        warn!("received abort signal. aborting process...");
        *STATE.mode.lock().await = Some(ShutdownMode::Abort);
        STATE.mode_changed.notify_waiters();
    });
}

/// Attempts to perform graceful shutdown the entire process without
/// the user or the host trigger the shutdown signal.
pub async fn shutdown(mode: ShutdownMode) {
    match mode {
        ShutdownMode::Graceful => {
            warn!("requested shutdown. performing graceful shutdown...");
        }
        ShutdownMode::Abort => {
            warn!("requested abort process. performing aborting process...");
        }
    }
    *STATE.mode.lock().await = Some(mode);
    STATE.mode_changed.notify_waiters();
}

pub async fn get_shutdown_mode() -> Option<ShutdownMode> {
    *STATE.mode.lock().await
}

#[must_use]
pub fn is_shutted_down() -> bool {
    let value = STATE.mode.try_lock().ok().map(|v| v.is_some());
    value.unwrap_or(false)
}

pub fn graceful() -> WaitForShutdownFuture {
    WaitForShutdownFuture {
        future: STATE.mode_changed.notified(),
        mode: ShutdownMode::Graceful,
    }
}

pub fn aborted() -> WaitForShutdownFuture {
    WaitForShutdownFuture {
        future: STATE.mode_changed.notified(),
        mode: ShutdownMode::Abort,
    }
}

/////////////////////////////////////////////////////////////////
static STATE: LazyLock<State> = LazyLock::new(|| State {
    catch_signals_guard: OnceLock::new(),
    mode: Mutex::new(None),
    mode_changed: Notify::new(),
});

struct State {
    pub(crate) catch_signals_guard: OnceLock<()>,
    pub(crate) mode: Mutex<Option<ShutdownMode>>,
    pub(crate) mode_changed: Notify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    Graceful,
    Abort,
}

#[allow(clippy::expect_used)]
mod signals {
    #[cfg(target_family = "unix")]
    use tokio::signal::unix::{signal, SignalKind};

    #[cfg(not(target_family = "unix"))]
    pub async fn abort() {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    }

    #[cfg(target_family = "unix")]
    pub async fn abort() {
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

        let mut sigquit = signal(SignalKind::quit()).expect("failed to install SIGQUIT handler");
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
            _ = sigquit.recv() => {},
        };
    }

    #[cfg(not(target_family = "unix"))]
    pub async fn graceful() {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    }

    #[cfg(target_family = "unix")]
    pub async fn graceful() {
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        };
    }
}
