use eden_bot_definitions::cmds::local_guild::PayerRegister;
use eden_db::{forms::InsertPayerForm, schema::Payer};
use eden_utils::Result;
use twilight_mention::Mention;
use twilight_model::{channel::message::MessageFlags, guild::Permissions};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::interaction::{
    cmds::{CommandContext, RunCommand},
    embeds, LocalGuildContext,
};

impl RunCommand for PayerRegister {
    async fn run(&self, ctx: &CommandContext<'_>) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx)?;
        ctx.defer(true).await?;

        let mut conn = ctx.bot.db_connection().await?;
        let existing_payer = Payer::from_id(&mut conn, ctx.author.id).await?;
        if existing_payer.is_some() {
            let embed = embeds::error::custom("Cannot register as a payer", None)
                .description("You're already a payer.")
                .build();

            let data = InteractionResponseDataBuilder::new()
                .embeds(vec![embed])
                .flags(MessageFlags::EPHEMERAL)
                .build();

            ctx.respond(data).await?;
            return Ok(());
        }

        // TODO: Admin approval system
        if needs_admin_approval(&ctx) {
            todo!()
        }

        let form = InsertPayerForm::builder()
            .id(ctx.author.id)
            .name(&ctx.author.name)
            .java_username(&self.java_username)
            .bedrock_username(self.bedrock_username.as_deref())
            .build();

        Payer::insert(&mut conn, form).await?;

        // TODO: Guide new payers on how to be a good payer or maybe we can have rules in some channel
        let data = InteractionResponseDataBuilder::new()
            .content(format!(
                "Welcome to the payers club, {}!",
                ctx.author.id.mention()
            ))
            .build();

        ctx.respond(data).await
    }
}

fn needs_admin_approval<'a, 'b, T>(ctx: &LocalGuildContext<'a, 'b, T>) -> bool {
    let permissions = ctx.member_permissions();

    let is_guild_owner = permissions == Permissions::all();
    let can_self_register = ctx.bot.settings.bot.guild.allow_self_payer_registration;

    is_guild_owner || can_self_register
}
