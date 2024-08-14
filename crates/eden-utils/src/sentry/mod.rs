use error_stack::{Frame, FrameKind};
use sentry::protocol::{Event, Exception, Map, Stacktrace};
use sqlx::types::Uuid;
use std::backtrace::Backtrace;
use std::panic::Location;
use tracing::warn;

use crate::{
    error::{GuildErrorCategory, UserErrorCategory},
    sql::SqlErrorExt,
    twilight::{error::TwilightHttpErrorExt, tags::DiscordHttpErrorInfo},
    Error, ErrorCategory,
};

mod internal;

pub fn capture_error_with_id<C>(error: &Error<C>) -> Uuid {
    sentry::Hub::with(|hub| {
        let event = event_from_error(error);
        let id = event.event_id;
        hub.capture_event(event);
        id
    })
}

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

    let mut extra = Map::new();
    let output = strip_ansi_escapes::strip_str(format!("{error}"));
    extra.insert(format!("report"), serde_json::Value::String(output));

    let last_chunk_id = chunks.len() - 1;
    for (id, chunk) in chunks.into_iter().enumerate() {
        let is_head = id == last_chunk_id;
        if is_head {
            exceptions.push(exception_from_chunk(error, chunk));
        }
        extra.insert(
            format!("frame[{id}].attachments"),
            serialize_attachments(chunk),
        );
    }

    event.exception = exceptions.into();
    event.level = sentry::Level::Error;
    event.extra = extra;
    event
}

fn exception_from_chunk<C>(error: &Error<C>, chunk: &[&Frame]) -> Exception {
    let module = chunk
        .iter()
        .filter_map(|v| {
            v.downcast_ref::<Location<'static>>()
                .or_else(|| v.downcast_ref::<Location<'_>>())
        })
        .next()
        .map(|v| v.file().to_string());

    let context = chunk
        .iter()
        .filter_map(|v| match v.kind() {
            FrameKind::Context(n) => Some(n),
            _ => None,
        })
        .next()
        .unwrap();

    let stacktrace = chunk
        .iter()
        .filter_map(|v| v.downcast_ref::<Backtrace>())
        .next()
        .and_then(|v| sentry_backtrace::parse_stacktrace(&format!("{v:#}")))
        .map(omit_internal_error_traces);

    Exception {
        ty: category_to_exception_type(error),
        value: Some(context.to_string()),
        module,
        stacktrace,
        raw_stacktrace: None,
        thread_id: std::thread::current()
            .name()
            .map(str::to_owned)
            .or_else(|| Some(String::from("<unknown thread>")))
            .map(|v| v.into()),
        ..Default::default()
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
                    function.contains("capwat_error::") || function.contains("error_stack");

                if contains_error_trace {
                    return None;
                }
            }
            Some(v)
        })
        .collect::<Vec<_>>();

    stacktrace
}

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

fn serialize_attachments(frames: &[&Frame]) -> serde_json::Value {
    let data = self::internal::SerializeAttachmentList::new(frames);
    match serde_json::to_value(data) {
        Ok(data) => data,
        Err(error) => {
            warn!(%error, "failed to serialize attachments while capturing error to Sentry");
            serde_json::Value::Null
        }
    }
}
