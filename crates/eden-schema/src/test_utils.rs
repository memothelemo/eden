use chrono::NaiveDate;
use eden_utils::error::exts::ResultExt;
use eden_utils::Result;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::{
    InsertAdminForm, InsertBillForm, InsertIdentityForm, InsertPayerApplicationForm,
    InsertPayerForm, InsertPaymentForm,
};
use crate::payment::{PaymentData, PaymentMethod};
use crate::types::{Admin, Bill, Identity, Payer, PayerApplication, Payment, User};

pub async fn generate_payer_application(conn: &mut sqlx::PgConnection) -> Result<PayerApplication> {
    let user_id = Id::new(12345678);
    let name = "poopyy";
    let java_username = "fooooo";
    let answer = "I like strawberry pies";

    let form = InsertPayerApplicationForm::builder()
        .user_id(user_id)
        .name(&name)
        .java_username(java_username)
        .bedrock_username(None)
        .answer(answer)
        .icon_url("https://example.com")
        .build();

    PayerApplication::insert(conn, form).await.anonymize_error()
}

#[must_use]
pub fn generate_mynt_payment() -> PaymentMethod {
    PaymentMethod::Mynt {
        name: Some("John Doe".into()),
        phone_number: None,
        proof_image_url: Some("https://192.168.0.1/images/jo/hn/doe/payments/1".into()),
        reference_number: None,
    }
}

#[must_use]
pub fn generate_paypal_payment() -> PaymentMethod {
    PaymentMethod::PayPal {
        name: Some("John Doe".into()),
        proof_image_url: Some("https://192.168.0.1/images/jo/hn/doe/payments/1".into()),
        transaction_id: None,
    }
}

pub async fn generate_payment(
    conn: &mut sqlx::PgConnection,
    bill_id: i64,
    payer_id: Id<UserMarker>,
) -> Result<Payment> {
    let form = InsertPaymentForm::builder()
        .bill_id(bill_id)
        .payer_id(payer_id)
        .data(
            PaymentData::builder()
                .method(generate_mynt_payment())
                .build(),
        )
        .build();

    Payment::insert(conn, form).await.anonymize_error()
}

pub async fn generate_identity(
    conn: &mut sqlx::PgConnection,
    payer_id: Id<UserMarker>,
) -> Result<Identity> {
    let form = InsertIdentityForm::builder()
        .payer_id(payer_id)
        .name(Some("dummy"))
        .uuid(None)
        .build();

    Identity::insert(conn, form).await.anonymize_error()
}

pub async fn generate_identity_with_name(
    conn: &mut sqlx::PgConnection,
    payer_id: Id<UserMarker>,
    name: &str,
) -> Result<Identity> {
    let form = InsertIdentityForm::builder()
        .payer_id(payer_id)
        .name(Some(name))
        .uuid(None)
        .build();

    Identity::insert(conn, form).await.anonymize_error()
}

pub async fn generate_payer(conn: &mut sqlx::PgConnection) -> Result<Payer> {
    let form = InsertPayerForm::builder()
        .id(Id::new(2345678))
        .name("foo")
        .java_username("foo123")
        .build();

    Payer::insert(conn, form).await.anonymize_error()
}

pub async fn generate_user(conn: &mut sqlx::PgConnection) -> Result<User> {
    User::insert(conn, Id::new(2345678)).await.anonymize_error()
}

pub async fn generate_admin(conn: &mut sqlx::PgConnection) -> Result<Admin> {
    let form = InsertAdminForm::builder()
        .id(Id::new(613425648685547541))
        .name(Some("admin"))
        .build();

    Admin::insert(conn, form).await.anonymize_error()
}

pub async fn generate_bill(conn: &mut sqlx::PgConnection) -> Result<Bill> {
    let created_by = Id::new(123456);
    let currency = "PHP";
    let deadline = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
    let price = Decimal::from_f64(20.).unwrap();

    let form = InsertBillForm::builder()
        .created_by(created_by)
        .currency(currency)
        .deadline(deadline)
        .price(price)
        .build();

    Bill::insert(conn, form).await.anonymize_error()
}
