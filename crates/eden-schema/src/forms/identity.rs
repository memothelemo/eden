use twilight_model::id::{marker::UserMarker, Id};
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertIdentityForm<'a> {
    pub payer_id: Id<UserMarker>,
    pub name: Option<&'a str>,
    pub uuid: Option<Uuid>,
}
