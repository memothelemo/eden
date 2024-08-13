use eden_schema::types::{PayerApplication, User};
use eden_utils::error::exts::IntoEdenResult;
use eden_utils::types::ProtectedString;
use eden_utils::Result;
use sqlx::types::Uuid;
use strum_macros::Display;
use tokio::sync::Mutex;
use tracing::warn;
use twilight_model::id::marker::{ChannelMarker, MessageMarker, UserMarker};
use twilight_model::id::Id;

use crate::interactions::state::{
    AnyStatefulCommand, CommandTriggerAction, StatefulCommandTrigger,
};
use crate::interactions::util::local_guild::{
    clear_member_lr_reactions, clear_reactions, react_lr_emojis, render_payer_application_embed,
};
use crate::util::http::request_for_model;
use crate::Bot;

#[derive(Debug)]
pub struct PayerApplicationPendingState {
    channel_id: Id<ChannelMarker>,
    current: Mutex<Uuid>,
    interaction_token: ProtectedString,
    invoker_id: Id<UserMarker>,
    message_id: Id<MessageMarker>,
}

impl PayerApplicationPendingState {
    #[must_use]
    pub fn new(
        channel_id: Id<ChannelMarker>,
        current: Uuid,
        interaction_token: &str,
        invoker_id: Id<UserMarker>,
        message_id: Id<MessageMarker>,
    ) -> Self {
        Self {
            channel_id,
            current: Mutex::new(current),
            interaction_token: ProtectedString::new(interaction_token),
            invoker_id,
            message_id,
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    #[strum(serialize = "left")]
    Previous,
    #[strum(serialize = "right")]
    Next,
}

impl AnyStatefulCommand for PayerApplicationPendingState {
    #[tracing::instrument(skip(bot))]
    async fn on_trigger(
        &self,
        bot: &Bot,
        trigger: StatefulCommandTrigger,
    ) -> eden_utils::Result<CommandTriggerAction> {
        let (user_id, message_id, direction) = match trigger {
            StatefulCommandTrigger::ReactedLeftArrow(user_id, message_id) => {
                (user_id, message_id, Direction::Previous)
            }
            StatefulCommandTrigger::ReactedRightArrow(user_id, message_id) => {
                (user_id, message_id, Direction::Next)
            }
            _ => return Ok(CommandTriggerAction::Nothing),
        };

        if user_id != self.invoker_id || message_id != self.message_id {
            return Ok(CommandTriggerAction::Nothing);
        }

        let result = self.shift_to_another_app(bot, direction).await;
        match result {
            Ok(true) => Ok(CommandTriggerAction::Continue),
            Ok(false) => Ok(CommandTriggerAction::Done),
            Err(error) => {
                if !bot.is_sentry_enabled() {
                    warn!(%error, "unable to shift current pending application to the {direction}");
                }

                let developer_mode = self
                    .has_user_enable_developer_mode(bot)
                    .await
                    .unwrap_or_else(|_| false);

                clear_reactions(bot, self.channel_id, self.message_id).await?;

                let data = crate::interactions::util::from_error(
                    false,
                    developer_mode,
                    bot.is_sentry_enabled(),
                    &error,
                );
                let embeds = data.embeds.unwrap_or_default();

                bot.interaction()
                    .update_response(self.interaction_token.expose())
                    .content(data.content.as_deref())
                    .unwrap()
                    .embeds(Some(&embeds))
                    .unwrap()
                    .await
                    .into_eden_error()?;

                Ok(CommandTriggerAction::Done)
            }
        }
    }

    #[tracing::instrument(skip(bot))]
    async fn on_timed_out(&self, bot: &Bot) -> eden_utils::Result<()> {
        let request = bot
            .http
            .update_message(self.channel_id, self.message_id)
            .content(Some("**Cancelled because of inactivity.**"))
            .unwrap();

        clear_reactions(bot, self.channel_id, self.message_id).await?;
        request_for_model(&bot.http, request).await?;

        Ok(())
    }
}

impl PayerApplicationPendingState {
    async fn has_user_enable_developer_mode(&self, bot: &Bot) -> Result<bool> {
        let mut conn = bot.db_read().await?;
        let user = User::get_or_insert(&mut conn, self.invoker_id).await?;
        Ok(user.developer_mode)
    }

    async fn shift_to_another_app(&self, bot: &Bot, direction: Direction) -> Result<bool> {
        let mut conn = bot.db_read().await?;
        let mut current_id = self.current.lock().await;

        let application = match direction {
            Direction::Previous => PayerApplication::before_pending(&mut conn, *current_id).await?,
            Direction::Next => PayerApplication::after_pending(&mut conn, *current_id).await?,
        };

        let Some(application) = application else {
            clear_reactions(bot, self.channel_id, self.message_id).await?;
            return Ok(false);
        };

        let embeds = vec![render_payer_application_embed(&application)];
        bot.http
            .update_message(self.channel_id, self.message_id)
            .embeds(Some(&embeds))
            .unwrap()
            .await
            .into_eden_error()?;

        let left = PayerApplication::before_pending(&mut conn, application.id)
            .await?
            .is_some();

        let right = PayerApplication::after_pending(&mut conn, application.id)
            .await?
            .is_some();

        *current_id = application.id;

        clear_member_lr_reactions(
            bot,
            self.channel_id,
            self.message_id,
            self.invoker_id,
            direction == Direction::Previous,
            direction == Direction::Next,
        )
        .await?;
        react_lr_emojis(bot, self.channel_id, self.message_id, left, right).await?;

        Ok(left || right)
    }
}
