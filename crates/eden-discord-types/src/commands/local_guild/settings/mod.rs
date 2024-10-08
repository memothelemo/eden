use twilight_interactions::command::{CommandModel, CreateCommand};

mod payer;
mod user;

pub use self::payer::*;
pub use self::user::*;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "settings",
    desc = "Commands to manage settings in this server",
    dm_permission = false
)]
pub enum SettingsCommand {
    #[command(name = "payer")]
    Payer(PayerSettingsCommand),
    #[command(name = "user")]
    User(UserSettingsCommand),
}
