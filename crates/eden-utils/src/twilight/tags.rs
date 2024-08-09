#[derive(Debug)]
pub enum DiscordHttpErrorInfo {
    Outage,
    Response(u64),
    Ratelimited,
    TimedOut,
    Unknown,
}

impl DiscordHttpErrorInfo {
    // https://discord.com/developers/docs/topics/opcodes-and-status-codes#json-json-error-codes
    #[must_use]
    pub fn has_missing_access(&self) -> bool {
        self.api_code().map(|v| v == 50001).unwrap_or_default()
    }

    // https://discord.com/developers/docs/topics/opcodes-and-status-codes#json-json-error-codes
    #[must_use]
    pub fn is_invalid_token(&self) -> bool {
        self.api_code().map(|v| v == 50014).unwrap_or_default()
    }

    #[must_use]
    pub fn api_code(&self) -> Option<u64> {
        match self {
            Self::Response(code) => Some(*code),
            _ => None,
        }
    }

    pub(crate) fn install_hook() {
        crate::Error::install_hook::<Self>(|this, ctx| match this {
            Self::Outage => {
                ctx.push_body("discord is down");
            }
            Self::Response(code) => ctx.push_body(format!("error code: {code:?}")),
            Self::Ratelimited => ctx.push_body("got ratelimited"),
            Self::TimedOut => ctx.push_body("request timed out"),
            Self::Unknown => {}
        });
    }
}
