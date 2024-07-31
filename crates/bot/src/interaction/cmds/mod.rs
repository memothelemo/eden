use eden_utils::error::{AnyResultExt, ErrorExt, ResultExt};
use eden_utils::Result;
use std::fmt::Debug;
use twilight_interactions::command::{CommandInputData, CommandModel, CreateCommand};
use twilight_model::guild::Permissions;

mod context;
mod local_guild;
mod ping;

use crate::{interaction::embeds, Bot};

pub use self::context::*;

#[allow(async_fn_in_trait)]
pub trait RunCommand: CreateCommand + CommandModel + Debug {
    /// Attempts to runs the command.
    ///
    /// This function assumes that you already sent the interaction
    /// response from Discord.
    ///
    /// If not done, the interaction will be considered as invalid
    /// and will result an error to the end user/invoker.
    async fn run(&self, ctx: &CommandContext<'_>) -> Result<()>;

    /// Required bot permissions from a guild to perform this command.
    ///
    /// Usually, the default is empty means that no permissions
    /// needed for the bot to perform something from this command.
    fn guild_permissions(&self) -> Permissions {
        Permissions::empty()
    }
}

async fn handle_command<'a, T: CommandModel + RunCommand>(
    ctx: &CommandContext<'a>,
    data: CommandInputData<'a>,
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
    use eden_bot_definitions::cmds;
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

    let name = ctx.command_name();

    let input: CommandInputData<'_> = ctx.data.clone().into();
    let result = match_commands!(ctx, input, [cmds::Ping, cmds::local_guild::PayerCommand]);

    if let Err(error) = result {
        // Emit warn event if it encounters internal error
        let is_user_error = error.get_category().is_user_error();
        if !is_user_error {
            tracing::warn!(%error, "command {name:?} failed");

            let author_id = ctx.interaction.author_id();
            let data = embeds::error::internal_error(&ctx.bot, &error, author_id);

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

pub async fn register(bot: &Bot) -> Result<()> {
    use eden_bot_definitions::cmds;
    macro_rules! create_cmds {
        [ $($command:ty),* $(,)? ] => {
            vec!{$( <$command as CreateCommand>::create_command().into(), )*}
        };
    }

    let interaction = bot.interaction();

    let global_commands = create_cmds![cmds::Ping];
    let local_guild_commands = create_cmds![cmds::local_guild::PayerCommand];

    let total_groups = global_commands.len() + local_guild_commands.len();
    let local_guild_id = bot.settings.bot.guild.id;

    tracing::debug!(
        "setting global commands with {} command group(s)",
        global_commands.len()
    );
    interaction
        .set_global_commands(&global_commands)
        .await
        .anonymize_error()?;

    tracing::debug!(
        "setting guild ({local_guild_id}) commands with {} command group(s)",
        local_guild_commands.len()
    );
    interaction
        .set_guild_commands(local_guild_id, &local_guild_commands)
        .await
        .anonymize_error()?;

    #[cfg(not(release))]
    tracing::info!("registered {total_groups} command group(s)");

    #[cfg(release)]
    println!("Registered {total_groups} command group(s)");

    Ok(())
}
