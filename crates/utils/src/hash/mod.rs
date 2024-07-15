pub mod bytes;
pub mod stream;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("Could not get hash of streaming data")]
pub struct HashError;
