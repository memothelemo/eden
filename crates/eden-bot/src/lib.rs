#![feature(new_uninit)]

mod context;
mod flags;
mod tasks;
#[cfg(test)]
mod tests;

pub mod events;
pub mod shard;

pub use self::context::Bot;
