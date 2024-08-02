#![feature(associated_type_defaults, closure_track_caller, error_iter)]

pub mod env;
pub mod error;
pub mod sql;
pub mod types;

pub use self::error::{Error, ErrorCategory, Result};
