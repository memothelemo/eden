use eden_db::forms::InsertPayerForm;
use eden_db::schema::Payer;
use eden_utils::error::AnyResultExt;
use eden_utils::{Error, Result, Sensitive};
use tracing::warn;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::channel::message::MessageFlags;
use twilight_model::guild::{PartialMember, Permissions};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::interaction::commands::{Command, CommandContext};
use crate::interaction::context::GuildAssertionError;
use crate::interaction::embeds;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "register",
    desc = "Allows you to register as a monthly payer",
    dm_permission = false
)]
pub struct PayerRegister {
    /// Your Minecraft Java Edition username.
    #[command(min_length = 2, max_length = 100)]
    java_username: Sensitive<String>,

    /// Your Minecraft Bedrock Edition username.
    #[command(min_length = 2, max_length = 100)]
    bedrock_username: Option<Sensitive<String>>,
}

impl Command for PayerRegister {
    #[tracing::instrument(skip(ctx))]
    async fn run(&self, ctx: &CommandContext<'_>) -> Result<()> {
        let member = ctx.guild_member()?;
        let Some(user) = member.user.as_ref() else {
            return Err(Error::unknown(GuildAssertionError))
                .attach_printable("could not get guild member's user info");
        };

        let mut conn = ctx.bot.db_connection().await?;
        if Payer::from_id(&mut conn, user.id).await?.is_some() {
            let embed = embeds::error("Cannot register as payer", None)
                .description("You're already a payer.")
                .build();

            let data = InteractionResponseDataBuilder::new()
                .embeds(vec![embed])
                .flags(MessageFlags::EPHEMERAL)
                .build();

            ctx.respond(data).await?;
            return Ok(());
        }

        if self.needs_admin_approval(ctx, member)? {
            todo!()
        }

        ctx.defer(true).await?;

        let form = InsertPayerForm::builder()
            .id(user.id)
            .name(&user.name)
            .java_username(&self.java_username)
            .bedrock_username(self.bedrock_username.as_deref())
            .build();

        // TODO: Guide users on how to be a good payer
        Payer::insert(&mut conn, form).await?;

        let data = InteractionResponseDataBuilder::new()
            .content(format!("Welcome to the payers club {}", user.id.mention()))
            .flags(MessageFlags::EPHEMERAL)
            .build();

        ctx.respond(data).await
    }
}

impl PayerRegister {
    #[allow(clippy::unused_self)]
    fn needs_admin_approval(
        &self,
        ctx: &CommandContext<'_>,
        member: &PartialMember,
    ) -> Result<bool> {
        // Maybe cache is the recommended option here...
        let calculator = ctx.bot.cache.permissions();
        let guild_id = ctx.guild_id()?;

        let Some(user_id) = member.user.as_ref().map(|v| v.id) else {
            return Err(Error::unknown(GuildAssertionError))
                .attach_printable("could not get guild member's user info");
        };

        let permissions = calculator.root(user_id, guild_id).ok();
        if permissions.is_none() && ctx.bot.is_cache_enabled() {
            warn!("could not calculate guild member permissions of {user_id:?}. using data from InteractionCreate");
        }

        let permissions = permissions
            .or(member.permissions)
            .unwrap_or_else(Permissions::empty);

        // If the bot administrator allows for guild administrators to register
        // themselves as payers, it can be registered automatically.
        let is_administrator = permissions.contains(Permissions::ADMINISTRATOR);
        let can_self_register = ctx.bot.settings.bot.guild.allow_self_payer_registration;
        let needs_admin_approval = !(is_administrator && can_self_register);

        Ok(needs_admin_approval)
    }
}
