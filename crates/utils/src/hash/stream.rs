use sha2::{Digest, Sha256, Sha384, Sha512};
use std::result::Result as StdResult;
use std::task::Poll;
use tokio::io::{AsyncRead, AsyncWrite};

use super::HashError;
use crate::error::{Result, ResultExt};

enum AsyncHashAlgorithm {
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

impl AsyncHashAlgorithm {
    #[must_use]
    fn finalize(self) -> Vec<u8> {
        match self {
            Self::Sha256(n) => n.finalize().to_vec(),
            Self::Sha384(n) => n.finalize().to_vec(),
            Self::Sha512(n) => n.finalize().to_vec(),
        }
    }
}

impl AsyncWrite for AsyncHashAlgorithm {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<StdResult<usize, std::io::Error>> {
        match self.get_mut() {
            Self::Sha256(hasher) => {
                hasher.update(buf);
            }
            Self::Sha384(hasher) => {
                hasher.update(buf);
            }
            Self::Sha512(hasher) => {
                hasher.update(buf);
            }
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<StdResult<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<StdResult<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}

macro_rules! make_hasher_fn {
    ($fn_name:ident, $hasher_name:ident) => {
        pub async fn $fn_name<R: AsyncRead + Unpin>(mut reader: R) -> Result<Vec<u8>, HashError> {
            let mut hasher = AsyncHashAlgorithm::$hasher_name($hasher_name::new());
            tokio::io::copy(&mut reader, &mut hasher)
                .await
                .change_context(HashError)?;

            Ok(hasher.finalize())
        }
    };
}

make_hasher_fn!(sha256, Sha256);
make_hasher_fn!(sha384, Sha384);
make_hasher_fn!(sha512, Sha512);
