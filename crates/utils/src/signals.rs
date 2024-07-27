// Borrowed from: https://github.com/memothelemo/kyoka/blob/master/crates/kyoka/src/util/mod.rs#L125
// License: AGPL-2.0
/// Cross-platform compatible function that yields the
/// current thread until one of the exit signals is triggered
/// by the operating system.
///
/// It allows programs to implement graceful shutdown to
/// prevent from any data loss or unexpected behavior to
/// the Discord bot (for example).
///
/// **For Windows / unsupported platforms**: It detects if `CTRL+C` is triggered
///
/// **For Unix systems**: It detects whether `SIGINT` or `SIGTERM` is triggered
#[cfg(not(unix))]
#[allow(clippy::expect_used)]
pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

// Borrowed from: https://github.com/memothelemo/kyoka/blob/master/crates/kyoka/src/util/mod.rs#L125
// License: AGPL-2.0
/// Cross-platform compatible function that yields the
/// current thread until one of the exit signals is triggered
/// by the operating system.
///
/// It allows programs to implement graceful shutdown to
/// prevent from any data loss or unexpected behavior to
/// the Discord bot (for example).
///
/// **For Windows**: It detects if `CTRL+C` is triggered
///
/// **For Unix systems**: It detects whether `SIGINT` or `SIGTERM` is triggered
#[cfg(unix)]
#[allow(clippy::expect_used)]
pub async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    };
}
