use eden_discord_types::commands::local_guild::PayerRegister;
use eden_schema::{
    forms::{InsertPayerApplicationForm, InsertPayerForm},
    types::{Payer, PayerApplication},
};
use eden_utils::{
    error::exts::{IntoEdenResult, ResultExt},
    sql::SqlErrorExt,
    Result,
};
use tracing::trace;
use twilight_mention::Mention;
use twilight_model::{
    application::interaction::application_command::CommandData, channel::message::Embed,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{CommandContext, RunCommand};
use crate::interactions::{embeds, record_local_guild_ctx, LocalGuildContext};

const ERROR_TITLE: &str = "Cannot register as payer";
const ALREADY_APPLIED_ERROR_DESC: &str = "**You already applied as a monthly contributor!**\n\nIf you want to see your application status, you may do so by running this command: `/payer application status`\n\nIf your application is still pending, please wait for admins to approve your application.\n\n**❤️      Good luck and I hope you'll be a monthly contributor!**";
const THANK_YOU_MESSAGE: &str = "**Nice! Thank you for applying for being a monthly contributor. I hope you will be accepted someday.**\n\nTake note that the server administrators will review your application and determine if they accept or revoke your application. If you want to see the status of your application, please do so by executing this command: `/payer app status`";

impl RunCommand for PayerRegister {
    #[tracing::instrument(skip(ctx), fields(ctx = tracing::field::Empty))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx).await?;
        record_local_guild_ctx!(ctx);

        let mut conn = ctx.bot.db_write().await?;
        trace!("checking if the user is already a payer");
        let payer = Payer::from_id(&mut conn, ctx.author.id).await?;
        if payer.is_some() {
            let embed = embeds::builders::error(ERROR_TITLE, None)
                .description("You're already a payer.")
                .build();

            return ctx.respond_with_embed(embed, false).await;
        }

        // Checking they have already applied for being a monthly contributor
        trace!("checking user's application");
        let application = PayerApplication::from_user_id(&mut conn, ctx.author.id).await?;
        if application.is_some() {
            let embed = embeds::builders::error(ERROR_TITLE, None)
                .description(ALREADY_APPLIED_ERROR_DESC)
                .build();

            return ctx.respond_with_embed(embed, true).await;
        }

        let result = if ctx.settings.payers.allow_self_register {
            try_insert_payer(&ctx, &mut conn, self).await
        } else {
            submit_application(&ctx, &mut conn, self).await
        };

        // duplicated usernme?
        if result.is_unique_violation() {
            let embed = generate_occupied_username_embed(self);
            return ctx.respond_with_embed(embed, false).await;
        }

        result?;
        conn.commit()
            .await
            .into_eden_error()
            .attach_printable("could not commit database transaction")?;

        Ok(())
    }
}

#[tracing::instrument(skip_all)]
async fn try_insert_payer(
    ctx: &LocalGuildContext<'_, CommandData>,
    conn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    args: &PayerRegister,
) -> Result<()> {
    trace!("inserting payer");

    let form = InsertPayerForm::builder()
        .id(ctx.author.id)
        .name(&ctx.author.name)
        .java_username(&args.java_username)
        .bedrock_username(args.bedrock_username.as_deref())
        .build();

    Payer::insert(conn, form).await?;

    // TODO: Guide new payers on how to be a good payer or maybe we can have rules in some channel
    let data = InteractionResponseDataBuilder::new()
        .content(format!(
            "Welcome to the payers club, {}!",
            ctx.author.id.mention()
        ))
        .build();

    ctx.respond(data).await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn submit_application(
    ctx: &LocalGuildContext<'_, CommandData>,
    conn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    args: &PayerRegister,
) -> Result<()> {
    trace!("submitting payer application");

    // Reason must be present in the arguments
    let Some(reason) = args.reason.as_ref() else {
        let embed = embeds::builders::error(ERROR_TITLE, None)
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

    trace!("inserting payer application");
    PayerApplication::insert(conn, form).await?;

    let embed = embeds::builders::success("Application submitted")
        .description(THANK_YOU_MESSAGE)
        .build();

    ctx.respond_with_embed(embed, false).await?;
    Ok(())
}

fn generate_occupied_username_embed(args: &PayerRegister) -> Embed {
    // Tell the user that either their Java or Bedrock usernames exist
    let mut desc = "Your chosen Java ".to_string();
    if args.bedrock_username.is_some() {
        desc.push_str(" or Bedrock ");
    }
    desc.push_str(" username exists in our monthly contributor records. Please try using ");
    desc.push_str(" your Java or Bedrock username something different else.\n\n");
    desc.push_str("Contact @memothelemo if you want to dispute this error.");

    embeds::builders::error(ERROR_TITLE, None)
        .description(desc)
        .build()
}
