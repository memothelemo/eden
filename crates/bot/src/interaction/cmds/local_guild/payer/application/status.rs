use chrono::{DateTime, Utc};
use eden_bot_definitions::cmds::local_guild::PayerApplicationStatus;
use eden_db::schema::PayerApplication;
use eden_utils::error::ResultExt;
use std::borrow::Cow;
use std::fmt::Write as _;
use twilight_util::builder::embed::EmbedFooterBuilder;

use crate::interaction::{
    cmds::{CommandContext, RunCommand},
    embeds, LocalGuildContext,
};

impl RunCommand for PayerApplicationStatus {
    async fn run(&self, ctx: &CommandContext<'_>) -> eden_utils::Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx)?;
        ctx.defer(true).await?;

        let mut conn = ctx.bot.db_connection().await?;
        let Some(application) = PayerApplication::from_user_id(&mut conn, ctx.author.id).await?
        else {
            let embed = embeds::error::custom("No application found", None)
                .description(NO_APPLICATION_ERROR_DESC)
                .build();

            return ctx.respond_with_embed(embed, true).await;
        };

        let mut content = String::from("**Status**: ");
        let mut footer = String::from("Updated: ");

        let embed = embeds::with_emoji('📋', "Application Status");
        let result = get_application_result(&application);
        match result {
            ApplicationResult::Pending => {
                write!(&mut footer, "None").anonymize_error()?;
                writeln!(&mut content, "🕑 Pending").anonymize_error()?;
                writeln!(&mut content).anonymize_error()?;
                writeln!(&mut content, "{PENDING_MESSAGE}").anonymize_error()?;
            }
            ApplicationResult::Passed { updated } => {
                write!(&mut footer, "{}", updated.to_rfc2822()).anonymize_error()?;
                writeln!(&mut content, "✅ Approved").anonymize_error()?;
                writeln!(&mut content).anonymize_error()?;
                write!(&mut content, "{APPROVED_MESSAGE}").anonymize_error()?;
            }
            ApplicationResult::Failed { reason, updated } => {
                write!(&mut footer, "{}", updated.to_rfc2822()).anonymize_error()?;

                let message = REJECTION_MESSAGE.replace("{INSERT_MESSAGE}", &reason);
                writeln!(&mut content, "❌ Rejected").anonymize_error()?;
                writeln!(&mut content).anonymize_error()?;
                write!(&mut content, "{message}").anonymize_error()?;
            }
        }

        let embed = embed
            .description(content)
            .footer(EmbedFooterBuilder::new(footer).build())
            .build();

        ctx.respond_with_embed(embed, true).await
    }
}

enum ApplicationResult<'a> {
    Pending,
    Passed {
        /// When this application was approved
        updated: DateTime<Utc>,
    },
    Failed {
        reason: Cow<'a, str>,
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
        ApplicationResult::Failed {
            reason: app
                .deny_reason
                .as_ref()
                .map(|v| Cow::Borrowed(v.as_str()))
                .unwrap_or_else(|| Cow::Owned(String::from("<no reason found>"))),
            updated,
        }
    }
}

const NO_APPLICATION_ERROR_DESC: &str = r"You haven't applied as a server monthly contributor yet.

If you want to apply as a server monthly contributor, please run this command and follow what is being asked:
```
/payer register
```";

const PENDING_MESSAGE: &str = r"Your application is pending for approval. Please wait for the server administrators to review your application. 

You may also check your application status periodically.";

const REJECTION_MESSAGE: &str = r"From the server administrators,

Thank you for putting your effort and time to polish and make your application stand out. However, we decided to reject your application because:

*{INSERT_MESSAGE}*

We apologize for rejecting your application, but look at the bright side, try to improve yourself, build your trust or yet make your application better and explain to us why you wanted to be a monthly contributor.

Once again, thank you for applying to be a server monthly contributor and we wish you good luck for your future endeavors.";

const APPROVED_MESSAGE: &str = r#"**CONGRATULATIONS!!!!!**

Woah! Server administrators approved your application! You're now a "monthly contributor" now.

Being as a monthly contributor, please read the guidelines about the obligations and rules of being a monthly contributor.

**Good luck on your journey, buddy!**"#;
