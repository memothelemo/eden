use thiserror::Error;

pub mod bytes {
    use sha2::Digest;

    macro_rules! make_hasher_fn {
        ($fn_name:ident, $hasher_name:ident) => {
            #[must_use]
            pub fn $fn_name<T: AsRef<[u8]>>(bytes: T) -> Vec<u8> {
                fn hash_impl(bytes: &[u8]) -> Vec<u8> {
                    let mut hasher = sha2::$hasher_name::new();
                    hasher.update(bytes);
                    hasher.finalize().to_vec()
                }
                hash_impl(bytes.as_ref())
            }
        };
    }

    make_hasher_fn!(sha256, Sha256);
    make_hasher_fn!(sha384, Sha384);
    make_hasher_fn!(sha512, Sha512);
}

pub mod file {
    use crate::error::exts::{IntoTypedError, ResultExt};
    use crate::Result;

    use super::HashError;
    use sha2::{Digest, Sha256, Sha384, Sha512};
    use std::path::Path;

    // 8 MB is enough for NVMe SSDs but not for hard drives or SATA SSDs.
    const STREAMING_THRESHOLD_SIZE: u64 = 1024 * 1024 * 8;

    macro_rules! make_hasher_fn {
        ($fn_name:ident, $hasher_name:ident) => {
            pub async fn $fn_name<P: AsRef<Path>>(path: P) -> Result<String, HashError> {
                let path = path.as_ref();
                let metadata = tokio::fs::metadata(path)
                    .await
                    .into_typed_error()
                    .change_context(HashError)
                    .attach_printable_lazy(|| {
                        format!("could not read file metadata for {}", path.display())
                    })?;

                if metadata.len() > STREAMING_THRESHOLD_SIZE {
                    let reader = tokio::fs::File::open(path)
                        .await
                        .into_typed_error()
                        .change_context(HashError)
                        .attach_printable_lazy(|| {
                            format!("could not open file for {}", path.display())
                        })?;

                    let algorithm = super::stream::Algorithm::$fn_name();
                    super::stream::HashStreamFuture::new(algorithm, reader, 1024 * 1024 * 8)
                        .await
                        .attach_printable_lazy(|| {
                            format!("could not get file hash for {}", path.display())
                        })
                } else {
                    let bytes = tokio::fs::read(path)
                        .await
                        .into_typed_error()
                        .change_context(HashError)
                        .attach_printable_lazy(|| {
                            format!("could not open file for {}", path.display())
                        })?;

                    let mut hasher = $hasher_name::new();
                    hasher.update(&bytes);

                    Ok(hex::encode(hasher.finalize()))
                }
            }
        };
    }

    make_hasher_fn!(sha256, Sha256);
    make_hasher_fn!(sha384, Sha384);
    make_hasher_fn!(sha512, Sha512);
}

pub mod stream {
    use sha2::{Digest, Sha256, Sha384, Sha512};

    const DEFAULT_BUF_SIZE: usize = 1024 * 8;

    pub fn sha256<R: AsyncRead + Unpin>(reader: R) -> HashStreamFuture<R> {
        HashStreamFuture::new(Algorithm::Sha256(Sha256::new()), reader, DEFAULT_BUF_SIZE)
    }

    pub fn sha384<R: AsyncRead + Unpin>(reader: R) -> HashStreamFuture<R> {
        HashStreamFuture::new(Algorithm::Sha384(Sha384::new()), reader, DEFAULT_BUF_SIZE)
    }

    pub fn sha512<R: AsyncRead + Unpin>(reader: R) -> HashStreamFuture<R> {
        HashStreamFuture::new(Algorithm::Sha512(Sha512::new()), reader, DEFAULT_BUF_SIZE)
    }

    use bytes::BytesMut;
    use pin_project_lite::pin_project;
    use std::{future::Future, task::Poll};
    use tokio::io::{AsyncRead, ReadBuf};

    use super::HashError;
    use crate::error::exts::{AnonymizeErrorInto, AnonymizedResultExt, IntoTypedError, ResultExt};

    pin_project! {
        #[must_use = "Futures are lazy. Use `.await` to get the hash from a streaming reader"]
        pub struct HashStreamFuture<R> {
            algorithm: Option<Algorithm>,
            buffer: BytesMut,
            #[pin]
            reader: R,
        }
    }

    #[allow(private_interfaces)]
    impl<R: AsyncRead + Unpin> HashStreamFuture<R> {
        pub(crate) fn new(algorithm: Algorithm, reader: R, buffer_size: usize) -> Self {
            Self {
                // based on https://git.savannah.gnu.org/gitweb/?p=coreutils.git;a=blob;f=src/digest.c;h=1a4cfd1fbfead794ea673d7cfd0ae02ec9b3006b;hb=HEAD#l323
                // but multiply it by 2 to improve the performance a bit.
                buffer: BytesMut::zeroed(buffer_size),
                algorithm: Some(algorithm),
                reader,
            }
        }
    }

    impl<R: AsyncRead + Unpin> Future for HashStreamFuture<R> {
        type Output = crate::Result<String, HashError>;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            let mut this = self.project();
            let mut buf = ReadBuf::new(&mut this.buffer);
            loop {
                match this.reader.as_mut().poll_read(cx, &mut buf) {
                    Poll::Ready(Ok(..)) if buf.filled().is_empty() => {
                        return Poll::Ready({
                            if let Some(algorithm) = this.algorithm.take() {
                                let hash = algorithm.finalize();
                                Ok(hex::encode(hash))
                            } else {
                                Err(HashError)
                                    .into_typed_error()
                                    .attach_printable("unexpected this.algorithm to be None")
                            }
                        })
                    }
                    Poll::Ready(Ok(())) => {
                        let Some(algorithm) = this.algorithm.as_mut() else {
                            let result = Err(HashError)
                                .into_typed_error()
                                .attach_printable("unexpected this.algorithm to be None");

                            return Poll::Ready(result);
                        };
                        algorithm.update(buf.filled());
                        buf.clear();
                    }
                    Poll::Ready(Err(error)) => {
                        let result = Err(error).anonymize_error_into().change_context(HashError);
                        return Poll::Ready(result);
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }

    pub(crate) enum Algorithm {
        Sha256(Sha256),
        Sha384(Sha384),
        Sha512(Sha512),
    }

    impl Algorithm {
        #[must_use]
        pub fn sha256() -> Self {
            Self::Sha256(Sha256::new())
        }

        #[must_use]
        pub fn sha384() -> Self {
            Self::Sha384(Sha384::new())
        }

        #[must_use]
        pub fn sha512() -> Self {
            Self::Sha512(Sha512::new())
        }

        fn update(&mut self, buf: &[u8]) {
            match self {
                Self::Sha256(hasher) => {
                    hasher.update(buf);
                }
                Self::Sha384(hasher) => {
                    hasher.update(buf);
                }
                Self::Sha512(hasher) => {
                    hasher.update(buf);
                }
            };
        }

        #[must_use]
        fn finalize(self) -> Vec<u8> {
            match self {
                Self::Sha256(n) => n.finalize().to_vec(),
                Self::Sha384(n) => n.finalize().to_vec(),
                Self::Sha512(n) => n.finalize().to_vec(),
            }
        }
    }
}

#[derive(Debug, Error)]
#[error("Could not get content hash")]
pub struct HashError;
