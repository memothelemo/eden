// TODO: Consider making a "eden-bot-defs" crate where we can define Eden's interaction types.
mod context;

pub mod autocomplete;
pub mod commands;
pub mod components;
pub mod embeds;

pub use self::context::InteractionContext;
