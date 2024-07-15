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
