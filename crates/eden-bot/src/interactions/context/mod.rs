use eden_utils::error::exts::{AnonymizedResultExt, IntoEdenResult, IntoTypedError, ResultExt};
use eden_utils::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::Span;
use twilight_model::channel::message::{AllowedMentions, Embed, MessageFlags};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::{ChannelMarker, MessageMarker, UserMarker};
use twilight_model::{application::interaction::Interaction, id::Id};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::events::EventContext;
use crate::shard::ShardHandle;
use crate::Bot;

mod local_guild;
pub use self::local_guild::*;

#[derive(Debug)]
pub struct InteractionContext<T> {
    pub bot: Bot,
    pub channel_id: Id<ChannelMarker>,
    pub data: T,
    pub interaction: Interaction,
    pub shard: ShardHandle,

    responded: AtomicBool,
}

impl<T> InteractionContext<T> {
    pub fn new(bot: Bot, ctx: &EventContext, data: T, interaction: &Interaction) -> Self {
        let Some(ref channel) = interaction.channel else {
            panic!("Ping interactions are not allowed to be used for creating contexts");
        };
        Self {
            bot,
            channel_id: channel.id,
            data,
            interaction: interaction.clone(),
            shard: ctx.shard.clone(),
            responded: AtomicBool::new(false),
        }
    }

    #[tracing::instrument(skip_all, fields(%ephemeral))]
    pub async fn defer(&self, ephemeral: bool) -> Result<Id<MessageMarker>> {
        let mut data = self.build_response();
        if ephemeral {
            data = data.flags(MessageFlags::EPHEMERAL);
        }

        let kind = InteractionResponseType::DeferredChannelMessageWithSource;
        self.send_response(Some(data.build()), kind)
            .await
            .attach_printable("could not respond with deferred message")
    }

    #[tracing::instrument(skip_all, fields(%ephemeral))]
    pub async fn respond_with_embed(
        &self,
        embed: Embed,
        ephemeral: bool,
    ) -> Result<Id<MessageMarker>> {
        let mut data = self.build_response().embeds(vec![embed]);
        if ephemeral {
            data = data.flags(MessageFlags::EPHEMERAL);
        }

        let kind = InteractionResponseType::DeferredChannelMessageWithSource;
        self.send_response(Some(data.build()), kind)
            .await
            .attach_printable("could not respond with embed")
    }

    pub async fn respond(&self, data: InteractionResponseData) -> Result<Id<MessageMarker>> {
        let kind = InteractionResponseType::ChannelMessageWithSource;
        self.send_response(Some(data), kind)
            .await
            .attach_printable("could not respond with message")
    }

    /// Gets the invoker's user id
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn invoker_id(&self) -> Id<UserMarker> {
        self.interaction
            .author_id()
            .expect("unexpected author id is None")
    }
}

impl<T> InteractionContext<T> {
    fn build_response(&self) -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    #[tracing::instrument(skip_all, fields(
        self.responded = tracing::field::Empty,
        response.kind = ?kind
    ))]
    async fn send_response(
        &self,
        data: Option<InteractionResponseData>,
        kind: InteractionResponseType,
    ) -> Result<Id<MessageMarker>> {
        let http = self.bot.interaction();
        let responded_earlier = self.responded.load(Ordering::Relaxed);

        let span = Span::current();
        if !span.is_disabled() {
            span.record("self.responded", tracing::field::display(responded_earlier));
        }

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
                follow_up = follow_up
                    .attachments(attachments)
                    .into_typed_error()
                    .anonymize_error()?;
            }

            if let Some(components) = &data.components {
                follow_up = follow_up
                    .components(components)
                    .into_typed_error()
                    .anonymize_error()?;
            }

            if let Some(content) = &data.content {
                follow_up = follow_up
                    .content(content)
                    .into_typed_error()
                    .anonymize_error()?;
            }

            if let Some(embeds) = &data.embeds {
                follow_up = follow_up
                    .embeds(embeds)
                    .into_typed_error()
                    .anonymize_error()?;
            }

            if let Some(flags) = data.flags {
                follow_up = follow_up.flags(flags);
            }

            if let Some(tts) = data.tts {
                follow_up = follow_up.tts(tts);
            }

            let message = follow_up
                .await
                .into_eden_error()
                .attach_printable("could not follow up response")?
                .model()
                .await
                .into_typed_error()?;

            Ok(message.id)
        } else {
            http.create_response(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse { kind, data },
            )
            .await
            .into_eden_error()
            .attach_printable("could not create interaction response")?;

            self.responded.store(true, Ordering::Relaxed);

            let request = http
                .response(&self.interaction.token)
                .await
                .into_eden_error()
                .attach_printable("could not fetch interaction respoonse")?
                .model()
                .await
                .into_typed_error()?;

            Ok(request.id)
        }
    }
}
