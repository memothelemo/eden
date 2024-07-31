use chrono::{DateTime, Utc};
use eden_bot_definitions::cmds::local_guild::PayerApplicationPending;
use eden_db::schema::PayerApplication;
use eden_utils::error::{AnyResultExt, ResultExt};
use futures::{FutureExt, TryFutureExt};
use std::fmt::Write as _;
use std::{borrow::Cow, future::IntoFuture};
use thiserror::Error;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::guild::Permissions;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

use crate::interaction::{
    cmds::{CommandContext, RunCommand},
    embeds, LocalGuildContext,
};

#[derive(Debug, Error)]
#[error("could not fetch interaction response")]
struct FetchResponseError;

impl RunCommand for PayerApplicationPending {
    async fn run(&self, ctx: &CommandContext<'_>) -> eden_utils::Result<()> {
        let embed = EmbedBuilder::new()
            .title("Hello!")
            .description("World!")
            .build();

        ctx.respond_with_embed(embed, false).await?;

        // Then, we can add some reactions
        let message = ctx
            .bot
            .interaction()
            .response(&ctx.interaction.token)
            .into_future()
            .map(|v| v.anonymize_error())
            .and_then(|v| v.model().map(|v| v.anonymize_error()))
            .await
            .transform_context(FetchResponseError)?;

        let http = &ctx.bot.http;
        http.create_reaction(
            message.channel_id,
            message.id,
            &RequestReactionType::Unicode { name: "⬅️" },
        )
        .await
        .anonymize_error()?;

        http.create_reaction(
            message.channel_id,
            message.id,
            &RequestReactionType::Unicode { name: "➡️" },
        )
        .await
        .anonymize_error()?;

        Ok(())
    }

    fn guild_permissions(&self) -> Permissions {
        Permissions::ADD_REACTIONS | Permissions::MANAGE_MESSAGES
    }
}
