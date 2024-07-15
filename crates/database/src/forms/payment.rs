use twilight_model::id::{marker::UserMarker, Id};
use typed_builder::TypedBuilder;

use crate::payment::PaymentData;

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertPaymentForm {
    pub payer_id: Id<UserMarker>,
    pub bill_id: i64,
    pub data: PaymentData,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct UpdatePaymentForm {
    pub data: PaymentData,
}
