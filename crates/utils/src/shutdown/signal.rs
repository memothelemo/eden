// Borrowed from: https://github.com/memothelemo/kyoka/blob/master/crates/kyoka/src/util/mod.rs#L125
#[cfg(not(target_family = "unix"))]
#[allow(clippy::expect_used)]
pub async fn abort_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[cfg(target_family = "unix")]
#[allow(clippy::expect_used)]
pub async fn abort_signal() {
    shutdown_signal().await;
}

#[cfg(not(target_family = "unix"))]
#[allow(clippy::expect_used)]
pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[cfg(target_family = "unix")]
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
