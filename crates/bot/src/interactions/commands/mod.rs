use async_trait::async_trait;
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::Result;
use std::fmt::Debug;
use twilight_interactions::command::{CommandInputData, CommandModel, CreateCommand};
use twilight_model::guild::Permissions;

pub mod chat;

use super::CommandContext;
use crate::Bot;

#[async_trait]
pub trait Command: CreateCommand + CommandModel + Debug {
    /// Permissions required to execute this command
    fn permissions() -> Permissions {
        Permissions::empty()
    }

    async fn run_command(&self, ctx: CommandContext<'_>) -> Result<()>;
}

#[tracing::instrument(skip_all, fields(
    ?ctx.app_permissions,
    ?ctx.data.id,
    ?ctx.data.guild_id,
    %ctx.data.name,
    ?ctx.data.kind,
))]
pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    macro_rules! match_commands {
        ($ctx:expr, $data:expr, [ $($command:ty),* $(,)? ]) => (match $ctx.data.name.as_str() {
            $( <$command>::NAME => <$command as CommandModel>::from_interaction($data)
                .attach_printable_lazy(|| format!("could not parse {:?} command from interaction", <$command>::NAME))?
                .run_command($ctx)
                .await
                .attach_printable_lazy(|| format!("command {:?} failed", <$command>::NAME))?, )*
            unknown => {
                tracing::warn!("unknown command: {unknown:?}");
                return Ok(());
            }
        });
    }

    let input: CommandInputData<'_> = ctx.data.clone().into();
    match_commands!(ctx, input, [chat::ping::Ping]);

    Ok(())
}

//////////////////////////////////////////////////////////////////////////////
macro_rules! create_cmds {
    [ $($command:ty),* $(,)? ] => {
        vec!{$( <$command as CreateCommand>::create_command().into(), )*}
    };
}

impl Bot {
    pub async fn register_commands(&self) -> Result<()> {
        let interaction = self.interaction();
        let commands = create_cmds![chat::ping::Ping];

        tracing::debug!("setting global commands with {} command(s)", commands.len());
        interaction
            .set_global_commands(&commands)
            .await
            .anonymize_error()?;

        #[cfg(not(release))]
        tracing::info!("registered {} command(s)", commands.len());

        #[cfg(release)]
        println!("registered {} command(s)", commands.len());

        Ok(())
    }
}
