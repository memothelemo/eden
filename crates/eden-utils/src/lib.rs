#![feature(async_fn_track_caller, backtrace_frames, let_chains, thread_id_value)]

pub mod serial;

pub mod hash;
pub mod sql;

pub mod sentry;
pub mod twilight;

pub mod shutdown;
pub mod tokio;

pub mod aliases;
pub mod build;
pub mod env;
pub mod error;
pub mod time;
pub mod types;
pub mod vec;

pub use self::error::{Error, ErrorCategory, Result};
