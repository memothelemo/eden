use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::{Error, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::guild::PartialMember;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::shard::ShardContext;
use crate::Bot;

#[derive(Debug)]
pub struct InteractionContext<'a, T> {
    pub bot: Bot,
    pub channel_id: Id<ChannelMarker>,
    pub data: T,
    pub interaction: Interaction,
    pub shard: &'a ShardContext,

    pub(super) guild_id: Option<Id<GuildMarker>>,
    responded: AtomicBool,
}

impl<'a, T> InteractionContext<'a, T> {
    pub fn new(bot: Bot, interaction: &Interaction, data: T, shard: &'a ShardContext) -> Self {
        let Some(ref channel) = interaction.channel else {
            panic!("unexpected interaction.channel is None");
        };
        Self {
            bot,
            channel_id: channel.id,
            data,
            guild_id: interaction.guild_id,
            interaction: interaction.clone(),
            shard,
            responded: AtomicBool::new(false),
        }
    }

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

    pub async fn respond(&self, data: InteractionResponseData) -> Result<()> {
        let kind = InteractionResponseType::ChannelMessageWithSource;
        self.send_response(Some(data), kind)
            .await
            .attach_printable("could not respond with message")
    }
}

#[derive(Debug, Error)]
#[error("unexpected interaction occurred in non-guild channel")]
pub struct GuildAssertionError;

impl<'a, T> InteractionContext<'a, T> {
    /// Gets the information about the invoker as a guild member.
    ///
    /// This function assumes that the command is issued from a guild.
    #[must_use]
    pub fn guild_id(&self) -> Result<Id<GuildMarker>> {
        self.guild_id
            .as_ref()
            .ok_or_else(|| Error::unknown(GuildAssertionError))
            .copied()
    }

    /// Gets the information about the invoker as a guild member.
    ///
    /// This function assumes that the command is issued from a guild.
    #[must_use]
    pub fn guild_member(&self) -> Result<&PartialMember> {
        self.interaction
            .member
            .as_ref()
            .ok_or_else(|| Error::unknown(GuildAssertionError))
    }
}

impl<'a, T> InteractionContext<'a, T> {
    fn build_response(&self) -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    async fn send_response(
        &self,
        data: Option<InteractionResponseData>,
        kind: InteractionResponseType,
    ) -> Result<()> {
        let http = self.bot.interaction();
        let responded_earlier = self.responded.load(Ordering::SeqCst);
        if responded_earlier {
            let mut follow_up = http.create_followup(&self.interaction.token);
            let data = match data {
                Some(data) => data,
                None => panic!("cannot follow up response without data"),
            };

            if let Some(mentions) = &data.allowed_mentions {
                follow_up = follow_up.allowed_mentions(Some(mentions));
            }
            if let Some(attachments) = &data.attachments {
                follow_up = follow_up.attachments(attachments).anonymize_error()?;
            }
            if let Some(components) = &data.components {
                follow_up = follow_up.components(components).anonymize_error()?;
            }
            if let Some(content) = &data.content {
                follow_up = follow_up.content(content).anonymize_error()?;
            }
            if let Some(embeds) = &data.embeds {
                follow_up = follow_up.embeds(embeds).anonymize_error()?;
            }
            if let Some(flags) = data.flags {
                follow_up = follow_up.flags(flags);
            }
            if let Some(tts) = data.tts {
                follow_up = follow_up.tts(tts);
            }

            follow_up
                .await
                .attach_printable("could not follow up response")?;

            Ok(())
        } else {
            http.create_response(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse { kind, data },
            )
            .await
            .attach_printable("could not create interaction response")?;
            self.responded.store(true, Ordering::SeqCst);

            Ok(())
        }
    }
}
