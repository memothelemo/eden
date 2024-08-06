use eden_utils::types::Sensitive;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::choices::PaymentMethodOption;

mod application;
pub use self::application::*;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "payer",
    desc = "Commands to manage things as a monthly contributor",
    dm_permission = false
)]
pub enum PayerCommand {
    #[command(name = "app")]
    Application(PayerApplicationCommand),
    #[command(name = "register")]
    Register(PayerRegister),
    #[command(name = "test")]
    Test(PayerTest),
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "register",
    desc = "Allows you to register as a monthly contributor",
    dm_permission = false
)]
pub struct PayerRegister {
    /// Your Minecraft Java Edition username.
    #[command(min_length = 2, max_length = 100)]
    pub java_username: Sensitive<String>,

    /// Your Minecraft Bedrock Edition username.
    #[command(min_length = 2, max_length = 100)]
    pub bedrock_username: Option<Sensitive<String>>,

    /// Why do you want to be the part of payers club?
    #[command(min_length = 15, max_length = 5000)]
    pub reason: Option<Sensitive<String>>,
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(name = "test", desc = "Just a testing command", dm_permission = false)]
pub struct PayerTest {
    /// Your preferred payment method
    #[allow(unused)]
    pub method: PaymentMethodOption,
}
