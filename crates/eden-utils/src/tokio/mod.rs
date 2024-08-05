use std::future::Future;
use tokio::task::JoinHandle;

/// Spawns a new asynchronous task with a name if `#[cfg(tokio_unstable)]`
/// is enabled from `RUSTFLAGS`.
///
/// This is useful for monitoring tokio tasks with `tokio-console`.
#[allow(unexpected_cfgs)]
pub fn spawn<F, T>(_name: &str, future: F) -> JoinHandle<T>
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    #[cfg(tokio_unstable)]
    let handle = tokio::task::Builder::new()
        .name(_name)
        .spawn(future)
        .expect("tried to spawn task outside tokio");

    #[cfg(not(tokio_unstable))]
    let handle = tokio::spawn(future);
    handle
}
