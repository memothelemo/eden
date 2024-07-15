use error_stack::Report;
use std::error::Error;
use std::str::FromStr;
use thiserror::Error;

use crate::error::{ErrorCategory, ErrorExt as _, IntoError, Result};

#[derive(Debug, Error)]
#[error("Could not load environment variable")]
pub struct EnvError;

#[track_caller]
fn into_env_error(var: &'static str, error: dotenvy::Error) -> crate::Error<EnvError> {
    use dotenvy::Error as DotenvyError;
    use std::env::VarError;
    match error {
        // line parse is unlikely to throw an error here
        DotenvyError::Io(n) => n.into_eden_error().transform_context(EnvError),
        DotenvyError::EnvVar(VarError::NotPresent) => crate::Error::report(
            ErrorCategory::default(),
            Report::new(EnvError)
                .attach_printable(format!("{var:?} variable is required to set to run Eden.")),
        ),
        DotenvyError::EnvVar(VarError::NotUnicode(..)) => crate::Error::report(
            ErrorCategory::default(),
            Report::new(EnvError)
                .attach_printable(format!("{var:?} must contain valid UTF-8 text.")),
        ),
        _ => unimplemented!(),
    }
}

#[track_caller]
pub fn var(key: &'static str) -> Result<String, EnvError> {
    dotenvy::var(key).map_err(|v| into_env_error(key, v))
}

#[track_caller]
pub fn var_opt(key: &'static str) -> Result<Option<String>, EnvError> {
    use dotenvy::Error as DotenvyError;
    use std::env::VarError;
    match dotenvy::var(key) {
        Ok(n) => Ok(Some(n)),
        Err(DotenvyError::EnvVar(VarError::NotPresent)) => Ok(None),
        Err(other) => Err(into_env_error(key, other)),
    }
}

#[track_caller]
pub fn var_opt_parsed<T: FromStr>(key: &'static str) -> Result<Option<T>, EnvError>
where
    T::Err: Error + Send + Sync + 'static,
{
    let Some(value) = var_opt(key)? else {
        return Ok(None);
    };
    match value.parse() {
        Ok(n) => Ok(Some(n)),
        Err(error) => Err(crate::Error::any(ErrorCategory::default(), error)
            .transform_context(EnvError)
            .attach_printable(format!("could not parse value of {key:?} variable"))),
    }
}

#[track_caller]
pub fn var_parsed<T: FromStr>(key: &'static str) -> Result<T, EnvError>
where
    T::Err: Error + Send + Sync + 'static,
{
    let var = var(key)?;
    match var.parse() {
        Ok(n) => Ok(n),
        Err(error) => Err(crate::Error::any(ErrorCategory::default(), error)
            .transform_context(EnvError)
            .attach_printable(format!("could not parse value of {key:?} variable"))),
    }
}

#[track_caller]
pub fn list(key: &'static str) -> Result<Vec<String>, EnvError> {
    let values = match var_opt(key)? {
        None => vec![],
        Some(s) if s.is_empty() => vec![],
        Some(s) => s.split(',').map(str::trim).map(String::from).collect(),
    };
    Ok(values)
}

#[track_caller]
pub fn list_opt(key: &'static str) -> Result<Option<Vec<String>>, EnvError> {
    let values = var_opt(key)?.map(|s| {
        if s.is_empty() {
            vec![]
        } else {
            s.split(',').map(str::trim).map(String::from).collect()
        }
    });

    Ok(values)
}
