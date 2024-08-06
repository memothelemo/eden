use chrono::NaiveDate;
use rust_decimal::Decimal;
use twilight_model::id::{marker::UserMarker, Id};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertBillForm<'a> {
    pub created_by: Id<UserMarker>,
    pub currency: &'a str,
    pub deadline: NaiveDate,
    pub price: Decimal,
}

#[derive(Debug, Default, Clone, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct UpdateBillForm<'a> {
    pub currency: Option<&'a str>,
    pub deadline: Option<NaiveDate>,
    pub price: Option<Decimal>,
}
