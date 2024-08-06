use twilight_model::id::{marker::UserMarker, Id};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertPayerApplicationForm<'a> {
    pub user_id: Id<UserMarker>,
    pub name: &'a str,
    pub java_username: &'a str,
    pub bedrock_username: Option<&'a str>,
    pub answer: &'a str,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct UpdatePayerApplicationForm<'a> {
    pub accepted: bool,
    pub deny_reason: &'a str,
}
