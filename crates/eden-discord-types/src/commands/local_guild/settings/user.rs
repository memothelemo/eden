use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "user",
    desc = "Commands to manage settings for each user",
    dm_permission = false
)]
pub enum UserSettingsCommand {
    #[command(name = "developer_mode")]
    DeveloperMode(UserSettingsDeveloperMode),
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "developer_mode",
    desc = "Modifies or gets 'developer mode' option",
    dm_permission = false
)]
pub struct UserSettingsDeveloperMode {
    /// Whether to set developer mode to true or not.
    pub set: Option<bool>,
}
