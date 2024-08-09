use pin_project_lite::pin_project;
use std::future::Future;
use std::task::Poll;
use tokio::sync::futures::Notified;

use super::{ShutdownMode, STATE};

pin_project! {
    #[must_use]
    pub struct WaitForShutdownFuture {
        #[pin]
        pub(super) future: Notified<'static>,
        pub(super) mode: ShutdownMode,
    }
}

impl Future for WaitForShutdownFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let matched = if let Some(mode) = STATE.mode.try_lock().ok() {
            mode.as_ref().map(|v| *v == self.mode).unwrap_or(false)
        } else {
            false
        };

        if matched {
            return Poll::Ready(());
        }

        let mut this = self.project();
        loop {
            match this.future.as_mut().poll(cx) {
                Poll::Ready(..) => {
                    let matched = if let Some(mode) = STATE.mode.try_lock().ok() {
                        mode.as_ref().map(|v| *v == *this.mode).unwrap_or(false)
                    } else {
                        false
                    };

                    if matched {
                        return Poll::Ready(());
                    }

                    this.future.set(STATE.mode_changed.notified());
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
