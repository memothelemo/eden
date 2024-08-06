use twilight_model::id::{marker::UserMarker, Id};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertAdminForm<'a> {
    pub id: Id<UserMarker>,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct UpdateAdminForm<'a> {
    pub name: &'a str,
}
