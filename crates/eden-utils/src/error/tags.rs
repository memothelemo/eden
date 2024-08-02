#[derive(Debug, PartialEq, Eq)]
pub struct Suggestion(&'static str);

impl Suggestion {
    #[must_use]
    pub const fn new(message: &'static str) -> Self {
        Self(message)
    }
}

impl Suggestion {
    pub(crate) fn install_hook() {
        crate::Error::install_hook::<Self>(move |this, ctx| {
            ctx.push_body(format!("suggestion: {}", this.0));
        });
    }
}
