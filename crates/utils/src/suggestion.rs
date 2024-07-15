use error_stack::Report;

#[derive(Debug, PartialEq, Eq)]
pub struct Suggestion(&'static str);

impl Suggestion {
    #[must_use]
    pub const fn new(message: &'static str) -> Self {
        Self(message)
    }

    pub fn install_hooks() {
        Report::install_debug_hook::<Suggestion>(|value, context| {
            context.push_body(format!("suggestion: {}", value.0));
        });
    }
}
