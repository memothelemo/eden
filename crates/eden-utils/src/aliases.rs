use std::sync::LazyLock;

// I cannot use the name `____` inside this repository so I
// called it Mynt in the mean time

/// Loaded alias name of Mynt.
///
/// This value can be overriden by setting the environment variable
/// `EDEN_MYNT_ALIAS` to something else.
pub static MYNT_NAME: LazyLock<String> = LazyLock::new(|| {
    let resolved_name = crate::env::var_opt("EDEN_MYNT_ALIAS").ok().and_then(|v| v);
    resolved_name.unwrap_or_else(|| String::from("Mynt"))
});

/// Loaded alias name of Mynt but in lowercase.
///
/// This value can be overriden by setting the environment variable
/// `EDEN_MYNT_ALIAS` to something else.
pub static MYNT_NAME_LOWERCASE: LazyLock<String> = LazyLock::new(|| MYNT_NAME.to_lowercase());

#[allow(clippy::let_underscore_must_use)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mynt_alias_vars() {
        let _ = crate::env::var("_");

        std::env::set_var("EDEN_MYNT_ALIAS", "Maya");
        assert_eq!(&*MYNT_NAME, "Maya");
        assert_eq!(&*MYNT_NAME_LOWERCASE, "maya");
    }
}
