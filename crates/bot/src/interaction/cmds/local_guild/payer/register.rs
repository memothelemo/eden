use eden_bot_definitions::cmds::local_guild::PayerRegister;
use eden_db::{
    forms::{InsertPayerApplicationForm, InsertPayerForm},
    schema::{Payer, PayerApplication},
};
use eden_utils::{
    error::{sql::SqlErrorExt, ResultExt},
    Result,
};
use twilight_mention::Mention;
use twilight_model::{channel::message::Embed, guild::Permissions};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::interaction::{
    cmds::{CommandContext, RunCommand},
    embeds, LocalGuildContext,
};

impl RunCommand for PayerRegister {
    async fn run(&self, ctx: &CommandContext<'_>) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx)?;
        ctx.defer(false).await?;

        let mut conn = ctx.bot.db_transaction().await?;
        let existing_payer = Payer::from_id(&mut conn, ctx.author.id).await?;
        if existing_payer.is_some() {
            let embed = embeds::error::custom(ERROR_TITLE, None)
                .description("You're already a payer.")
                .build();

            return ctx.respond_with_embed(embed, true).await;
        }

        // Checking they have already applied for being a monthly contributor
        let existing_application = PayerApplication::from_user_id(&mut conn, ctx.author.id).await?;
        if existing_application.is_some() {
            let embed = embeds::error::custom(ERROR_TITLE, None)
                .description(ALREADY_APPLIED_ERROR_DESC)
                .build();

            return ctx.respond_with_embed(embed, true).await;
        }

        if needs_admin_approval(&ctx) {
            process_register_application(self, conn, &ctx).await
        } else {
            try_insert(self, conn, &ctx).await
        }
    }
}

async fn process_register_application<'a, 'b, T>(
    args: &PayerRegister,
    mut conn: sqlx::Transaction<'_, sqlx::Postgres>,
    ctx: &LocalGuildContext<'a, 'b, T>,
) -> Result<()> {
    // Reason must be present in the arguments
    let Some(reason) = args.reason.as_ref() else {
        let embed = embeds::error::custom(ERROR_TITLE, None)
            .description("Please input your reason why you wanted to be part of the payers' club.")
            .build();

        ctx.respond_with_embed(embed, true).await?;
        return Ok(());
    };

    let form = InsertPayerApplicationForm::builder()
        .user_id(ctx.author.id)
        .name(&ctx.author.name)
        .java_username(&args.java_username)
        .bedrock_username(args.bedrock_username.as_deref())
        .answer(&reason)
        .build();

    let result = PayerApplication::insert(&mut conn, form).await;
    if result.has_unique_violation() {
        let embed = generate_occupied_username_embed::<T>(args);
        return ctx.respond_with_embed(embed, true).await;
    } else {
        result?;
    }

    conn.commit()
        .await
        .attach_printable("could not commit database transaction")?;

    let embed = embeds::success("Application submitted")
        .description(THANK_YOU_MESSAGE)
        .build();

    ctx.respond_with_embed(embed, false).await
}

async fn try_insert<'a, 'b, T>(
    args: &PayerRegister,
    mut conn: sqlx::Transaction<'_, sqlx::Postgres>,
    ctx: &LocalGuildContext<'a, 'b, T>,
) -> Result<()> {
    let form = InsertPayerForm::builder()
        .id(ctx.author.id)
        .name(&ctx.author.name)
        .java_username(&args.java_username)
        .bedrock_username(args.bedrock_username.as_deref())
        .build();

    let result = Payer::insert(&mut conn, form).await;
    if result.has_unique_violation() {
        let embed = generate_occupied_username_embed::<T>(args);
        return ctx.respond_with_embed(embed, true).await;
    } else {
        result?;
    }

    conn.commit()
        .await
        .attach_printable("could not commit database transaction")?;

    // TODO: Guide new payers on how to be a good payer or maybe we can have rules in some channel
    let data = InteractionResponseDataBuilder::new()
        .content(format!(
            "Welcome to the payers club, {}!",
            ctx.author.id.mention()
        ))
        .build();

    ctx.respond(data).await
}

fn generate_occupied_username_embed<'a, 'b, T>(args: &PayerRegister) -> Embed {
    // Tell the user that either their Java or Bedrock usernames exist
    let mut desc = "Your chosen Java ".to_string();
    if args.bedrock_username.is_some() {
        desc.push_str(" or Bedrock ");
    }
    desc.push_str(" username exists in our payer records. Please try using your Java or ");
    desc.push_str(" Bedrock username something different else.\n\n");
    desc.push_str("Contact @memothelemo if you want to dispute this error.");

    embeds::error::custom(ERROR_TITLE, None)
        .description(desc)
        .build()
}

fn needs_admin_approval<'a, 'b, T>(ctx: &LocalGuildContext<'a, 'b, T>) -> bool {
    let permissions = ctx.member_permissions();

    let is_guild_owner = permissions == Permissions::all();
    let can_self_register = ctx.bot.settings.bot.guild.allow_self_payer_registration;

    !(is_guild_owner || can_self_register)
}

const ERROR_TITLE: &str = "Cannot register as a payer";
const ALREADY_APPLIED_ERROR_DESC: &str = "**You already applied as a monthly contributor!**

If you want to see your application status, you may do so by running this command: `/payer application status`

If your application is still pending, please wait for admins to approve your application.

**❤️      Good luck and I hope you'll be a monthly contributor!**";

const THANK_YOU_MESSAGE: &str = "**Nice! Thank you for applying for being a monthly contributor. I hope you will be accepted someday.**

Take note that the server administrators will review your application and determine if they accept or revoke your application. If you want to see the status of your application, please do so by executing this command: `/payer app status`";
