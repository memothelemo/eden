use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "payer",
    desc = "Commands to manage settings for monthly contributors",
    dm_permission = false
)]
pub enum PayerSettingsCommand {
    #[command(name = "allow_self_registration")]
    AllowSelfRegistration(PayerSettingsAllowSelfRegistration),
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "allow_self_registration",
    desc = "Modifies or gets 'Allow self registration' option",
    dm_permission = false
)]
pub struct PayerSettingsAllowSelfRegistration {
    /// Whether anyone can register as a monthly contributor
    /// without admin approval
    pub set: Option<bool>,
}
