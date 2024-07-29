// Some parts of the code are copied from: https://github.com/CircuitSacul/Starboard-4/blob/main/src/interactions/context.rs
//
// Licensed under MIT License
use eden_utils::error::{AnyResultExt, ErrorExt, ResultExt};
use eden_utils::{Error, ErrorCategory, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::shard::ShardContext;
use crate::Bot;

pub type CommandContext<'a> = InteractionContext<'a, CommandData>;

// mimicking from sparkle_convenience
#[derive(Debug)]
pub struct InteractionContext<'a, T> {
    pub app_permissions: Permissions,
    pub bot: Bot,
    pub data: T,
    pub interaction: Interaction,
    pub shard: &'a ShardContext,
    responded: AtomicBool,
}

impl<'a, T> InteractionContext<'a, T> {
    pub fn new(bot: Bot, interaction: Interaction, data: T, shard: &'a ShardContext) -> Self {
        let app_permissions = interaction
            .app_permissions
            .unwrap_or_else(|| Permissions::empty());

        Self {
            app_permissions,
            bot,
            data,
            interaction,
            shard,
            responded: AtomicBool::new(false),
        }
    }
}

impl<'a, T> InteractionContext<'a, T> {
    pub async fn defer(&self, ephemeral: bool) -> Result<()> {
        let mut data = self.build_response();
        if ephemeral {
            data = data.flags(MessageFlags::EPHEMERAL);
        }

        let kind = InteractionResponseType::DeferredChannelMessageWithSource;
        self.send_response(Some(data.build()), kind)
            .await
            .attach_printable("could not respond with deferred message")
    }

    pub async fn edit(&self, data: InteractionResponseData) -> Result<()> {
        self.send_response(Some(data), InteractionResponseType::UpdateMessage)
            .await
            .attach_printable("could not update message")
    }

    pub async fn respond(&self, data: InteractionResponseData) -> Result<()> {
        self.send_response(
            Some(data),
            InteractionResponseType::ChannelMessageWithSource,
        )
        .await
        .attach_printable("could not respond with message")
    }

    async fn send_response(
        &self,
        data: Option<InteractionResponseData>,
        kind: InteractionResponseType,
    ) -> Result<()> {
        let http = self.bot.interaction();
        if self.responded.load(Ordering::SeqCst) {
            let mut followup = http.create_followup(&self.interaction.token);
            let data = match data {
                None => panic!("followed up messages must have a response data"),
                Some(data) => data,
            };

            if let Some(mentions) = &data.allowed_mentions {
                followup = followup.allowed_mentions(Some(mentions));
            }

            if let Some(attachments) = &data.attachments {
                followup = followup
                    .attachments(attachments)
                    .anonymize_error()
                    .attach_printable("some data.attachments have invalid data")?;
            }

            if let Some(components) = &data.components {
                followup = followup
                    .components(components)
                    .anonymize_error()
                    .attach_printable("some data.components have invalid data")?;
            }

            if let Some(content) = &data.content {
                followup = followup
                    .content(content)
                    .anonymize_error()
                    .attach_printable("data.content has invalid data")?;
            }

            if let Some(embeds) = &data.embeds {
                followup = followup
                    .embeds(embeds)
                    .anonymize_error()
                    .attach_printable("some data.embeds has invalid data")?;
            }

            if let Some(flags) = data.flags {
                followup = followup.flags(flags);
            }

            if let Some(tts) = data.tts {
                followup = followup.tts(tts);
            }

            followup
                .await
                .attach_printable("could not respond interaction with follow up message")?;

            Ok(())
        } else {
            http.create_response(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse { data, kind },
            )
            .await
            .attach_printable("could not create interaction response")?;

            http.response(&self.interaction.token)
                .await
                .attach_printable("could not respond interaction")?;

            Ok(())
        }
    }
}

impl<'a, T> InteractionContext<'a, T> {
    /// Unlike [`InteractionContext::has_permissions`], this function will return
    /// an error if the bot doesn't have required permissions from the argument.
    pub fn check_permissions(&self, required: Permissions) -> Result<()> {
        #[derive(Debug, Error)]
        #[error("bot has missing permissions")]
        struct MissingPermissionsError;

        let missing = required.difference(self.app_permissions);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(Error::any(ErrorCategory::Guild, MissingPermissionsError)
                .attach(MissingPermissions(missing)))
        }
    }

    /// Checks if the bot has required permissions from the argument.
    pub const fn has_permissions(&self, required: Permissions) -> bool {
        let missing = required.difference(self.app_permissions);
        !missing.is_empty()
    }

    pub fn build_response(&self) -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MissingPermissions(Permissions);

impl MissingPermissions {
    pub fn install_error_hook() {
        use eden_utils::error::Error;
        Error::install_hook::<Self>(|value, context| {
            context.push_body(format!("missing permissions: {:?}", value.0));
        });
    }

    #[must_use]
    pub const fn get(self) -> Permissions {
        self.0
    }
}
