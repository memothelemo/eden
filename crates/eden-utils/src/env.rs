use std::error::Error as StdError;
use std::str::FromStr;
use thiserror::Error;

use crate::error::exts::{IntoError, IntoTypedError, ResultExt};
use crate::Result;

#[derive(Debug, Error)]
#[error("Could not load environment variable")]
pub struct LoadEnvError;

#[track_caller]
pub fn var(key: &'static str) -> Result<String, LoadEnvError> {
    dotenvy::var(key).map_err(|v| (v, key).into_eden_error())
}

#[track_caller]
pub fn var_opt(key: &'static str) -> Result<Option<String>, LoadEnvError> {
    use std::env::VarError;
    match dotenvy::var(key) {
        Ok(n) => Ok(Some(n)),
        Err(dotenvy::Error::EnvVar(VarError::NotPresent)) => Ok(None),
        Err(other) => Err((other, key).into_eden_error()),
    }
}

#[track_caller]
pub fn var_opt_parsed<T: FromStr>(key: &'static str) -> Result<Option<T>, LoadEnvError>
where
    T::Err: StdError + Send + Sync + 'static,
{
    let Some(value) = var_opt(key)? else {
        return Ok(None);
    };
    match value.parse() {
        Ok(n) => Ok(Some(n)),
        Err(error) => Err(error)
            .into_typed_error()
            .change_context(LoadEnvError)
            .attach_printable(format!("could not parse value of {key:?} variable")),
    }
}

#[track_caller]
pub fn var_parsed<T: FromStr>(key: &'static str) -> Result<T, LoadEnvError>
where
    T::Err: StdError + Send + Sync + 'static,
{
    let value = var(key)?;
    match value.parse() {
        Ok(n) => Ok(n),
        Err(error) => Err(error)
            .into_typed_error()
            .change_context(LoadEnvError)
            .attach_printable(format!("could not parse value of {key:?} variable")),
    }
}

#[track_caller]
pub fn list(key: &'static str) -> Result<Vec<String>, LoadEnvError> {
    let values = match var_opt(key)? {
        None => vec![],
        Some(s) if s.is_empty() => vec![],
        Some(s) => s.split(',').map(str::trim).map(String::from).collect(),
    };
    Ok(values)
}

#[track_caller]
pub fn list_opt(key: &'static str) -> Result<Option<Vec<String>>, LoadEnvError> {
    let values = var_opt(key)?.map(|s| {
        if s.is_empty() {
            vec![]
        } else {
            s.split(',').map(str::trim).map(String::from).collect()
        }
    });

    Ok(values)
}
