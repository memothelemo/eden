use thiserror::Error;

#[derive(Debug, Error)]
#[error("Could not perform query")]
pub struct QueryError;
