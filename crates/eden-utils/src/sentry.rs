use error_stack::{AttachmentKind, Frame, FrameKind, Report};
use sentry::protocol::{Event, Exception, Map, Mechanism};
use sentry_backtrace::Stacktrace;
use std::backtrace::Backtrace;
use std::panic::Location;
use tracing_error::SpanTrace;

use crate::error::{GuildErrorCategory, UserErrorCategory};
use crate::sql::SqlErrorExt;
use crate::twilight::{error::TwilightHttpErrorExt, tags::DiscordHttpErrorInfo};
use crate::{Error, ErrorCategory};

#[track_caller]
pub fn capture_error_with_id<C>(error: &Error<C>) -> sentry::types::Uuid {
    sentry::Hub::with(|hub| {
        let event = event_from_error(error);
        let id = event.event_id;
        hub.capture_event(event);
        id
    })
}

#[track_caller]
pub fn capture_error<C>(error: &Error<C>) {
    sentry::Hub::with(|hub| hub.capture_event(event_from_error(error)));
}

fn event_from_error<C>(error: &Error<C>) -> Event<'static> {
    let mut event = Event::default();
    let mut exceptions = Vec::new();

    let frames = error.report.frames().collect::<Vec<_>>();
    let chunks = frames
        .split_inclusive(|v| matches!(v.kind(), FrameKind::Context(..)))
        .collect::<Vec<_>>();

    let last_chunk_id = chunks.len() - 1;
    for (id, chunk) in chunks.into_iter().enumerate() {
        // We also want to include span information so we can look into deeper
        // without relying on the provided backtrace
        let is_head = id == last_chunk_id;
        if is_head {
            make_span_exception(error, &mut exceptions);
        }
        exceptions.push(exception_from_chunk(error, chunk, is_head));
    }

    event.exception = exceptions.into();
    event.level = sentry::Level::Error;
    event
}

fn make_span_exception<C>(error: &Error<C>, exceptions: &mut Vec<Exception>) {
    let Some(span) = error.report.downcast_ref::<SpanTrace>() else {
        return;
    };

    let mut exception = Exception {
        ty: "Span Tree".into(),
        ..Default::default()
    };

    let mut frames = Vec::new();
    span.with_spans(|metadata, fields| {
        let mut map = Map::new();
        map.insert(
            "fields".to_string(),
            serde_json::Value::String(fields.into()),
        );

        let frame = sentry::protocol::Frame {
            function: Some(metadata.name().into()),
            abs_path: metadata.file().map(|v| v.into()),
            lineno: metadata.line().map(|v| v as u64),
            module: Some(format!("{}::{}", metadata.target(), metadata.name())),
            vars: map,
            ..Default::default()
        };
        frames.push(frame);
        true
    });

    exception.stacktrace = Some(Stacktrace {
        frames,
        ..Default::default()
    });
    exceptions.push(exception);
}

#[track_caller]
fn category_to_exception_type<C>(error: &Error<C>) -> String {
    if let Some(info) = error.discord_http_error_info() {
        return match info {
            DiscordHttpErrorInfo::Outage => {
                "HTTP request is performed while Discord is down".into()
            }
            DiscordHttpErrorInfo::Response(code) => {
                format!("Got Discord HTTP JSON error code {code}")
            }
            DiscordHttpErrorInfo::Ratelimited => format!("Got ratelimited from Discord"),
            DiscordHttpErrorInfo::Unknown => "Discord HTTP request error".into(),
            DiscordHttpErrorInfo::TimedOut => "Discord HTTP request timed out".into(),
        };
    }

    if error.is_pool_error() {
        return format!("Unhealthy database pool");
    }

    if error.is_statement_timed_out() {
        return format!("Query statement got timed out");
    }

    match &error.category {
        ErrorCategory::Guild(cat) => match cat {
            GuildErrorCategory::MissingChannelPermissions(..) => {
                format!("Bot lacked permissions in guild channel")
            }
            GuildErrorCategory::MissingGuildPermissions(..) => {
                format!("Bot lacked guild permissions")
            }
            GuildErrorCategory::NotInLocalGuild => {
                format!("User tried to perform local guild only operation")
            }
        },
        ErrorCategory::User(cat) => match cat {
            UserErrorCategory::MissingPermissions => {
                format!("User tried to perform with insufficient permissions")
            }
        },
        ErrorCategory::Unknown => "Error".into(),
    }
}

fn exception_from_chunk<C>(error: &Error<C>, chunk: &[&Frame], is_head: bool) -> Exception {
    let module = chunk
        .iter()
        .filter_map(|v| {
            v.downcast_ref::<Location<'static>>()
                .or_else(|| v.downcast_ref::<Location<'_>>())
        })
        .next()
        .map(|v| v.file().to_string());

    let stacktrace = chunk
        .iter()
        .filter_map(|v| v.downcast_ref::<Backtrace>())
        .next()
        .and_then(|v| sentry_backtrace::parse_stacktrace(&format!("{v:#}")))
        .map(omit_internal_error_traces);

    let context = chunk
        .iter()
        .filter_map(|v| match v.kind() {
            FrameKind::Context(n) => Some(n),
            _ => None,
        })
        .next()
        .unwrap();

    Exception {
        ty: if is_head {
            category_to_exception_type(error)
        } else {
            context.to_string()
        },
        value: is_head.then(|| context.to_string()),
        module,
        stacktrace,
        raw_stacktrace: None,
        thread_id: std::thread::current()
            .name()
            .map(str::to_owned)
            .or_else(|| Some(String::from("<unknown thread>")))
            .map(|v| v.into()),
        mechanism: Some(Mechanism {
            ty: "<unknown>".into(),
            data: serialize_attachments(chunk),
            ..Default::default()
        }),
    }
}

fn omit_internal_error_traces(mut stacktrace: Stacktrace) -> Stacktrace {
    let mut new_frames = Vec::new();
    std::mem::swap(&mut new_frames, &mut stacktrace.frames);

    stacktrace.frames = new_frames
        .into_iter()
        .filter_map(|v| {
            if let Some(function) = v.function.as_ref() {
                let contains_error_trace =
                    function.contains("eden_utils::error::") || function.contains("error_stack");

                if contains_error_trace {
                    return None;
                }
            }
            Some(v)
        })
        .collect::<Vec<_>>();

    stacktrace
}

fn serialize_attachments(frames: &[&Frame]) -> Map<String, serde_json::Value> {
    let mut map = Map::new();
    let attachments = frames.iter().filter_map(|v| match v.kind() {
        FrameKind::Attachment(n) => Some((v, n)),
        _ => None,
    });

    let mut fmt_context = error_stack::fmt::Config::load(true);
    let fmt_context = fmt_context.context();

    let mut serialized = Vec::new();
    for (frame, attachment) in attachments {
        // Exclude location and span trace
        let has_builtin_hooks = frame
            .downcast_ref::<Location<'static>>()
            .map(|_| true)
            .or_else(|| frame.downcast_ref::<SpanTrace>().map(|_| true))
            .unwrap_or_default();

        if has_builtin_hooks {
            continue;
        }

        match attachment {
            AttachmentKind::Opaque(..) => {
                let body = Report::invoke_debug_format_hook(|hooks| hooks.call(frame, fmt_context))
                    .then(|| fmt_context.take_body())
                    .unwrap_or_default();

                serialized.extend(body);
            }
            AttachmentKind::Printable(data) => {
                serialized.push(data.to_string());
            }
            _ => todo!(),
        }
    }

    map.insert("attachments".into(), serialized.into());
    map
}

// #[track_caller]
// fn event_from_error<C>(error: &Error<C>) -> Event<'static> {
//     let mut event = Event::default();
//     let frames = {
//         let mut frames = Vec::new();
//         for frame in error.report.frames() {
//             frames.push(frame);
//         }
//         frames
//     };

//     let mut exceptions = Vec::new();
//     let chunks = frames.split_inclusive(|v| matches!(v.kind(), FrameKind::Context(..)));
//     for chunk in chunks {
//         exceptions.push(exception_from_chunk(chunk));
//     }

//     exceptions.last_mut().map(|v| {
//         v.ty = get_main_exception_type(&error).into();
//         let context = error
//             .report
//             .frames()
//             .filter_map(|v| match v.kind() {
//                 FrameKind::Context(cn) => Some(cn),
//                 _ => None,
//             })
//             .next()
//             .map(|v| v.to_string());

//         v.value = context;
//         v
//     });
//     event.exception = exceptions.into();
//     event
// }

// #[allow(clippy::unwrap_used)]
// #[track_caller]
// fn exception_from_chunk(frames: &[&Frame]) -> Exception {
//     let module = frames
//         .iter()
//         .filter_map(|v| {
//             v.downcast_ref::<Location<'static>>()
//                 .or_else(|| v.downcast_ref::<Location<'_>>())
//         })
//         .next()
//         .map(|v| v.file().to_string());

//     let stacktrace = frames
//         .iter()
//         .filter_map(|v| v.downcast_ref::<Backtrace>())
//         .next()
//         .and_then(|v| sentry_backtrace::parse_stacktrace(&format!("{v:#}")))
//         .map(|mut v| {
//             v.frames = Vec::new();
//             let frames = v.frames.into_iter().filter_map(|v| {
//                 if let Some(module) = v.module
//                     && module.contains("eden_utils::error::")
//                 {
//                     None
//                 } else {
//                     Some(v)
//                 }
//             });
//             v
//         });

//     let context = frames
//         .iter()
//         .filter_map(|v| match v.kind() {
//             FrameKind::Context(n) => Some(n),
//             _ => None,
//         })
//         .next()
//         .unwrap();

//     // then we can serialize into JSON if we want to
//     let mut data = Map::new();
//     let raw_attachments = frames.iter().filter_map(|v| match v.kind() {
//         FrameKind::Attachment(n) => Some((v, n)),
//         _ => None,
//     });

//     // let mut serialized_attachments = Vec::new();
//     let mut fmt_context = error_stack::fmt::Config::load(true);
//     let fmt_context = fmt_context.context();

//     let mut fmt_attachments = Vec::new();
//     for (frame, attachment) in raw_attachments {
//         // Exclude location and span trace
//         let has_builtin_hooks = frame
//             .downcast_ref::<Location<'static>>()
//             .map(|_| true)
//             .or_else(|| frame.downcast_ref::<SpanTrace>().map(|_| true))
//             .unwrap_or_default();

//         if has_builtin_hooks {
//             continue;
//         }

//         match attachment {
//             AttachmentKind::Opaque(..) => {
//                 let body = Report::invoke_debug_format_hook(|hooks| hooks.call(frame, fmt_context))
//                     .then(|| fmt_context.take_body())
//                     .unwrap_or_default();

//                 fmt_attachments.extend(body);
//             }
//             AttachmentKind::Printable(data) => {
//                 fmt_attachments.push(data.to_string());
//             }
//             _ => todo!(),
//         }
//     }
//     data.insert("attachments".into(), fmt_attachments.into());

//     Exception {
//         ty: context.to_string(),
//         value: None,
//         module,
//         stacktrace,
//         raw_stacktrace: None,
//         thread_id: std::thread::current()
//             .name()
//             .map(str::to_owned)
//             .or_else(|| Some(String::from("<unknown thread>")))
//             .map(|v| v.into()),
//         mechanism: Some(Mechanism {
//             data,
//             ..Default::default()
//         }),
//     }
// }
