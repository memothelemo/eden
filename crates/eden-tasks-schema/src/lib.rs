#![feature(const_trait_impl, const_try)]

pub mod forms;
pub mod impls;
pub mod paged_queries;
pub mod types;

#[cfg(test)]
use sqlx::migrate::Migrator;

#[cfg(test)]
static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[cfg(test)]
pub(crate) mod test_utils;
