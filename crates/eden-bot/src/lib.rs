#![feature(new_uninit)]

mod context;
mod flags;
mod tasks;
#[cfg(test)]
mod tests;

pub use self::context::Bot;
