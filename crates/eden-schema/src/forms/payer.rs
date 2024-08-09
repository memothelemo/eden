use twilight_model::id::{marker::UserMarker, Id};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertPayerForm<'a> {
    pub id: Id<UserMarker>,
    pub name: &'a str,
    // this is really important, there must be at
    // least one identity per payer
    pub java_username: &'a str,
    #[builder(default)]
    pub bedrock_username: Option<&'a str>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct UpdatePayerForm<'a> {
    pub name: &'a str,
}
