use chrono::{DateTime, Utc};
use eden_discord_types::commands::local_guild::PayerApplicationStatus;
use eden_schema::types::PayerApplication;
use eden_utils::{error::exts::IntoTypedError, types::Sensitive, Result};
use std::borrow::Cow;
use std::fmt::Write as _;
use tracing::trace;
use twilight_util::builder::embed::EmbedFooterBuilder;

use crate::interactions::{
    commands::{CommandContext, RunCommand},
    LocalGuildContext,
};
use crate::interactions::{embeds, record_local_guild_ctx};

const NO_APPLICATION_ERROR_DESC: &str = "You haven't applied as a server monthly contributor yet.\n\nIf you want to apply as a server monthly contributor, please run this command and follow what is being asked:\n```/payer register```";
const PENDING_MESSAGE: &str = "Your application is pending for approval. Please wait for the server administrators to review your application.\n\nYou may also check your application status periodically.";
const REJECTION_MESSAGE: &str = "From the server administrators,\n\nThank you for putting your effort and time to polish and make your application stand out. However, we decided to reject your application because:\n\n*{INSERT_MESSAGE}*\n\nWe apologize for rejecting your application, but look at the bright side, try to improve yourself, build your trust or yet make your application better and explain to us why you wanted to be a monthly contributor.\n\nOnce again, thank you for applying to be a server monthly contributor and we wish you good luck for your future endeavors.";
const APPROVED_MESSAGE: &str = "**CONGRATULATIONS!!!!!**\n\nWoah! Server administrators approved your application! You're now a \"monthly contributor\" now.\n\nBeing as a monthly contributor, please read the guidelines about the obligations and rules of being a monthly contributor.\n\n**Good luck on your journey, buddy!**";

impl RunCommand for PayerApplicationStatus {
    #[tracing::instrument(skip_all, fields(ctx = tracing::field::Empty))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx).await?;
        record_local_guild_ctx!(ctx);

        let mut conn = ctx.bot.db_read().await?;

        trace!("fetching payer application");
        let Some(application) = PayerApplication::from_user_id(&mut conn, ctx.author.id).await?
        else {
            let embed = embeds::builders::error("No application found", None)
                .description(NO_APPLICATION_ERROR_DESC)
                .build();

            ctx.respond_with_embed(embed, false).await?;
            return Ok(());
        };

        let mut content = String::from("**Status**: ");
        let mut footer = String::from("Updated: ");

        let embed = embeds::builders::with_emoji('📋', "Application Status");
        let result = get_application_result(&application);

        // we need to let the user know that the time zone is in UTC
        trace!(?result, "got payer application result");
        match result {
            ApplicationResult::Pending => {
                write!(&mut footer, "None").into_typed_error()?;
                writeln!(&mut content, "🕑 Pending").into_typed_error()?;
                writeln!(&mut content).into_typed_error()?;
                writeln!(&mut content, "{PENDING_MESSAGE}").into_typed_error()?;
            }
            ApplicationResult::Passed { updated } => {
                write!(&mut footer, "{} (UTC)", updated.to_rfc2822()).into_typed_error()?;
                writeln!(&mut content, "✅ Approved").into_typed_error()?;
                writeln!(&mut content).into_typed_error()?;
                write!(&mut content, "{APPROVED_MESSAGE}").into_typed_error()?;
            }
            ApplicationResult::Failed { reason, updated } => {
                write!(&mut footer, "{} (UTC)", updated.to_rfc2822()).into_typed_error()?;

                let message = REJECTION_MESSAGE.replace("{INSERT_MESSAGE}", &reason.into_inner());
                writeln!(&mut content, "❌ Rejected").into_typed_error()?;
                writeln!(&mut content).into_typed_error()?;
                write!(&mut content, "{message}").into_typed_error()?;
            }
        }

        let embed = embed
            .description(content)
            .footer(EmbedFooterBuilder::new(footer).build())
            .build();

        ctx.respond_with_embed(embed, false).await?;
        Ok(())
    }
}

#[derive(Debug)]
enum ApplicationResult<'a> {
    Pending,
    Passed {
        /// When this application was approved
        updated: DateTime<Utc>,
    },
    Failed {
        reason: Sensitive<Cow<'a, str>>,
        /// When this application was revoked
        updated: DateTime<Utc>,
    },
}

fn get_application_result(app: &PayerApplication) -> ApplicationResult<'_> {
    let Some(accepted) = app.accepted else {
        return ApplicationResult::Pending;
    };

    let updated = app.updated_at.unwrap_or_else(Utc::now);
    if accepted {
        ApplicationResult::Passed { updated }
    } else {
        let reason = app
            .deny_reason
            .as_ref()
            .map(|v| Cow::Borrowed(v.as_str()))
            .unwrap_or_else(|| Cow::Owned(String::from("<no reason found>")));

        ApplicationResult::Failed {
            reason: Sensitive::new(reason),
            updated,
        }
    }
}
