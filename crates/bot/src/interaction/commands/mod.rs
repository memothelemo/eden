use chrono::Utc;
use eden_utils::error::{AnyResultExt, ErrorExt, ResultExt};
use eden_utils::Result;
use std::fmt::Debug;
use twilight_interactions::command::{CommandInputData, CommandModel, CreateCommand};
use twilight_model::guild::Permissions;
use twilight_util::builder::InteractionResponseDataBuilder;

use super::InteractionContext;
use crate::interaction::embeds;
use crate::Bot;

mod context;

pub mod chat;
pub mod choices;
pub use self::context::*;

#[allow(async_fn_in_trait)]
pub trait Command: CreateCommand + CommandModel + Debug {
    async fn run(&self, ctx: &CommandContext<'_>) -> Result<()>;

    /// Required bot permissions from a guild to run this command
    fn guild_permissions(&self) -> Permissions {
        Permissions::empty()
    }
}

async fn handle_command<T: CommandModel + Command>(
    ctx: &CommandContext<'_>,
    data: CommandInputData<'_>,
) -> Result<()> {
    let command = T::from_interaction(data).attach_printable_lazy(|| {
        format!("could not parse {:?} command from interaction", T::NAME)
    })?;

    // TODO: Check guild permissions. I guess...
    command
        .run(ctx)
        .await
        .attach_printable_lazy(|| format!("command {:?} failed", T::NAME))
}

pub async fn handle(ctx: CommandContext<'_>) -> Result<()> {
    tracing::debug!("received command {:?}", ctx.data.name);

    macro_rules! match_commands {
        ($ctx:expr, $data:expr, [ $($command:ty),* $(,)? ]) => (match $ctx.data.name.as_str() {
            $( <$command>::NAME => handle_command::<$command>(&$ctx, $data).await, )*
            unknown => {
                tracing::warn!("unknown command: {unknown:?}");
                Ok(())
            }
        });
    }

    let ran_at = Utc::now();
    let name = ctx.command_name();

    let input: CommandInputData<'_> = ctx.data.clone().into();
    let result = match_commands!(ctx, input, [chat::ping::Ping, chat::payer::PayerCommand]);

    if let Err(error) = result {
        // Emit warn event if it encounters internal error
        let is_user_error = error.get_category().is_user_error();
        if !is_user_error {
            tracing::warn!(%error, "command {name:?} failed");

            let embed = embeds::internal_error(ran_at).build();
            let data = InteractionResponseDataBuilder::new()
                .embeds(vec![embed])
                .build();

            ctx.respond(data)
                .await
                .attach_printable("could not respond command with internal error message")?;
        } else {
            tracing::debug!(%error, "command {name:?} failed because of human error");
        }
    } else {
        tracing::debug!("successfully ran command {name:?}");
    }

    Ok(())
}

macro_rules! create_cmds {
    [ $($command:ty),* $(,)? ] => {
        vec!{$( <$command as CreateCommand>::create_command().into(), )*}
    };
}

pub async fn register_commands(bot: &Bot) -> Result<()> {
    let interaction = bot.interaction();

    let global_commands = create_cmds![chat::ping::Ping];
    let guild_commands = create_cmds![chat::payer::PayerCommand];

    let total_groups = global_commands.len() + guild_commands.len();
    let guild_id = bot.settings.bot.guild.id;

    tracing::debug!(
        "setting global commands with {} command group(s)",
        global_commands.len()
    );
    interaction
        .set_global_commands(&global_commands)
        .await
        .anonymize_error()?;

    tracing::debug!(
        "setting guild ({guild_id}) commands with {} command group(s)",
        guild_commands.len()
    );
    interaction
        .set_guild_commands(guild_id, &guild_commands)
        .await
        .anonymize_error()?;

    #[cfg(not(release))]
    tracing::info!("registered {total_groups} command group(s)");

    #[cfg(release)]
    println!("Registered {total_groups} command group(s)");

    Ok(())
}
