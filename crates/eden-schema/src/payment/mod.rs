use chrono::NaiveDate;
use eden_utils::types::Sensitive;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use typed_builder::TypedBuilder;

mod mynt;
mod version;

pub use self::mynt::*;
pub use self::version::*;

#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder, PartialEq, Eq)]
pub struct PaymentData {
    #[serde(rename = "v")]
    #[builder(default)]
    pub version: PaymentDataVersion,
    pub method: PaymentMethod,
    #[builder(default)]
    pub status: PaymentStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum PaymentMethod {
    // Other name for the most popular e-wallet
    // providers in the Philippines.
    Mynt {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Sensitive<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        phone_number: Option<Sensitive<PHPhoneNumber>>,
        // Hosted somewhere using pict.rs
        #[serde(skip_serializing_if = "Option::is_none")]
        proof_image_url: Option<Sensitive<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reference_number: Option<Sensitive<String>>,
    },
    PayPal {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Sensitive<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        proof_image_url: Option<Sensitive<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        transaction_id: Option<Sensitive<String>>,
    },
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum PaymentStatus {
    Success,
    #[default]
    Pending,
    Failed {
        reason: String,
    },
    Refunded {
        reason: String,
    },
    // In case if the user cannot pay in time due
    // to special circumstances
    Void {
        next_payment: NaiveDate,
        reason: String,
    },
}
