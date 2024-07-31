use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "app",
    desc = "Commands to manage/view monthly contributor applications",
    dm_permission = false
)]
pub enum PayerApplicationCommand {
    #[command(name = "status")]
    Status(PayerApplicationStatus),
}

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "status",
    desc = "View the status of your monthly contributor application",
    dm_permission = false
)]
pub struct PayerApplicationStatus;
