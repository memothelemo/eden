// TODO: Fully integrate sentry with eden_utils::Error

// use error_stack::{AttachmentKind, FrameKind};
// use sentry::protocol::{Event, Exception};

// use crate::Error;

// pub fn event_from_error<C>(error: &Error<C>) -> Event<'static> {
//     let mut event = Event::default();
//     let frames = {
//         let mut frames = Vec::new();
//         for frame in error.report.frames() {
//             frames.push(frame);
//         }
//         frames.into_iter().rev()
//     };

//     let mut exceptions = Vec::new();
//     for frame in frames {
//         Exception {

//         }
//     }

//     todo!()
// }
