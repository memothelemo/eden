use once_cell::sync::{Lazy, OnceCell};
use pin_project_lite::pin_project;
use std::future::{Future, IntoFuture};
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};
use std::task::Poll;
use tokio::sync::{futures::Notified, Notify};

// This is not really memory efficient
//
// TODO: Find a way to minimize amount of fields needed for this struct.
pub struct State {
    pub(super) abort_notify: Arc<Notify>,
    pub(super) abort: AtomicBool,

    pub(super) graceful_notify: Arc<Notify>,
    pub(super) graceful: AtomicBool,

    pub(super) catch_signals_guard: OnceCell<()>,
}

pub static STATE: Lazy<State> = Lazy::new(|| State {
    abort_notify: Arc::new(Notify::new()),
    abort: AtomicBool::new(false),

    graceful_notify: Arc::new(Notify::new()),
    graceful: AtomicBool::new(false),

    catch_signals_guard: OnceCell::new(),
});

pin_project! {
    #[must_use = "ShutdownFuture is a future. Use `.await` to wait until graceful or abort shutdown has been triggered"]
    pub struct ShutdownFuture {
        #[pin]
        pub(super) notify: Notified<'static>,
        pub(super) is_graceful: bool,
    }
}

impl ShutdownFuture {
    pub fn graceful() -> Self {
        Self {
            notify: STATE.graceful_notify.notified().into_future(),
            is_graceful: true,
        }
    }

    pub fn abort() -> Self {
        Self {
            notify: STATE.abort_notify.notified().into_future(),
            is_graceful: false,
        }
    }
}

impl Future for ShutdownFuture {
    type Output = ();

    // FIXME: Is this efficient enough?
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let done = if self.is_graceful {
            &STATE.graceful
        } else {
            &STATE.abort
        }
        .load(Ordering::SeqCst);

        let this = self.project();
        if done {
            Poll::Ready(())
        } else {
            this.notify.poll(cx)
        }
    }
}
