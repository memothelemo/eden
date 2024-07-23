use chrono::{NaiveDate, TimeDelta, Utc};
use eden_utils::error::ResultExt;
use eden_utils::Result;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::{
    InsertAdminForm, InsertBillForm, InsertIdentityForm, InsertJobForm, InsertPayerForm,
    InsertPaymentForm,
};
use crate::payment::{PaymentData, PaymentMethod};
use crate::schema::{Admin, Bill, Identity, Job, JobData, JobPriority, Payer, Payment};

pub async fn generate_job(conn: &mut sqlx::PgConnection) -> Result<Job> {
    let form = InsertJobForm::builder()
        .name("foo")
        .deadline(Utc::now())
        .priority(JobPriority::default())
        .task(serde_json::json!({
            "currency": "PHP",
            "deadline": Utc::now(),
            "payer_id": "613425648685547541",
            "price": 15.0,
        }))
        .build();

    Job::insert(conn, form).await.anonymize_error()
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
        .name(Some("dummy".into()))
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
        .name(Some(name.into()))
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

pub async fn prepare_sample_jobs(conn: &mut sqlx::PgConnection) -> eden_utils::Result<()> {
    // prepare 5 sample deadlines
    let deadline_1 = Utc::now();
    let deadline_2 = deadline_1
        .checked_add_signed(TimeDelta::seconds(5))
        .unwrap();

    let deadline_3 = deadline_2
        .checked_add_signed(TimeDelta::seconds(3))
        .unwrap();

    let deadline_4 = deadline_3
        .checked_add_signed(TimeDelta::seconds(1))
        .unwrap();

    let deadline_5 = deadline_4
        .checked_add_signed(TimeDelta::milliseconds(500))
        .unwrap();

    // Then prepare these jobs for some reason :)
    let task = serde_json::json!({
        "currency": "PHP",
        "deadline": Utc::now(),
        "payer_id": "613425648685547541",
        "price": 15.0,
    });

    // Prepare a list of jobs (situation stuff)
    // - deadline_1 - high priority
    // - deadline_2 - low priority
    // - deadline_1 - medium priority
    // - deadline_3 - high priority and so on
    macro_rules! shorthand_insert {
        ($deadline:ident, $priority:ident) => {{
            Job::insert(
                conn,
                InsertJobForm::builder()
                    .deadline($deadline)
                    .name("foo")
                    .priority(JobPriority::$priority)
                    .data(JobData {
                        kind: "foo".into(),
                        inner: task.clone(),
                    })
                    .build(),
            )
            .await
            .anonymize_error()?;
        }};
    }

    shorthand_insert!(deadline_1, High);
    shorthand_insert!(deadline_3, Low);
    shorthand_insert!(deadline_4, High);
    shorthand_insert!(deadline_1, Low);
    shorthand_insert!(deadline_5, High);
    shorthand_insert!(deadline_2, Low);
    shorthand_insert!(deadline_5, Medium);
    shorthand_insert!(deadline_1, Medium);
    shorthand_insert!(deadline_3, High);
    shorthand_insert!(deadline_5, Low);
    shorthand_insert!(deadline_2, High);
    shorthand_insert!(deadline_4, Medium);
    shorthand_insert!(deadline_2, Medium);
    shorthand_insert!(deadline_3, Medium);
    shorthand_insert!(deadline_4, Low);

    Ok(())
}