use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::guild::Permissions;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "app",
    desc = "Commands to manage/view monthly contributor applications",
    dm_permission = false
)]
pub enum PayerApplicationCommand {
    #[command(name = "pending")]
    Pending(PayerApplicationPending),
    #[command(name = "status")]
    Status(PayerApplicationStatus),
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "pending",
    desc = "Shows all pending monthly contributor applications",
    dm_permission = false,
    default_permissions = "PayerApplicationPending::required_permissions"
)]
pub struct PayerApplicationPending;

impl PayerApplicationPending {
    fn required_permissions() -> Permissions {
        Permissions::ADMINISTRATOR
    }
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "status",
    desc = "View the status of your monthly contributor application",
    dm_permission = false
)]
pub struct PayerApplicationStatus;
