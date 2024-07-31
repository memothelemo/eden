use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::{Error, Result};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tracing::warn;
use twilight_model::channel::message::{AllowedMentions, Embed, MessageFlags};
use twilight_model::guild::{PartialMember, Permissions};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::user::User;
use twilight_model::{application::interaction::Interaction, id::Id};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::shard::SimplifiedShardContext;
use crate::{Bot, ShardContext};

pub struct InteractionContext<'shard_ctx, T> {
    pub bot: Bot,
    pub channel_id: Id<ChannelMarker>,
    pub data: T,
    pub interaction: Interaction,
    pub shard: &'shard_ctx ShardContext,

    responded: AtomicBool,
}

impl<'shard_ctx, T> InteractionContext<'shard_ctx, T> {
    pub fn new(
        bot: Bot,
        data: T,
        interaction: &Interaction,
        shard: &'shard_ctx ShardContext,
    ) -> Self {
        let Some(ref channel) = interaction.channel else {
            panic!("Ping interactions are not allowed to be used for creating contexts");
        };
        Self {
            bot,
            channel_id: channel.id,
            data,
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

    pub async fn respond_with_embed(&self, embed: Embed, ephemeral: bool) -> Result<()> {
        let mut data = self.build_response().embeds(vec![embed]);
        if ephemeral {
            data = data.flags(MessageFlags::EPHEMERAL);
        }

        let kind = InteractionResponseType::DeferredChannelMessageWithSource;
        self.send_response(Some(data.build()), kind)
            .await
            .attach_printable("could not respond with embed")
    }

    pub async fn respond(&self, data: InteractionResponseData) -> Result<()> {
        let kind = InteractionResponseType::ChannelMessageWithSource;
        self.send_response(Some(data), kind)
            .await
            .attach_printable("could not respond with message")
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

impl<'shard_ctx, T: Debug> Debug for InteractionContext<'shard_ctx, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InteractionContext")
            .field("bot", &self.bot)
            .field("channel_id", &self.channel_id)
            .field("data", &self.data)
            .field("interaction", &self.interaction)
            .field("responded", &self.responded.load(Ordering::SeqCst))
            .field("shard", &SimplifiedShardContext(&self.shard))
            .finish()
    }
}

/// Extension of [`InteractionContext`] but it contains special guild
/// data. This allows for easier access to configurations and data.
#[derive(Debug)]
pub struct LocalGuildContext<'ctx, 'shard_ctx, T> {
    /// User that invoked the interaction.
    pub author: &'ctx User,
    /// ID of the guild where the interaction was invoked.
    pub guild_id: &'ctx Id<GuildMarker>,
    /// Guild member object of the invoker.
    pub member: &'ctx PartialMember,
    /// Inner data of [`LocalGuildContext`].
    pub inner: &'ctx InteractionContext<'shard_ctx, T>,
}

#[derive(Debug, Error)]
#[error("unexpected interaction was not invoked in local guild channel")]
pub struct NotInLocalGuildError;

impl<'ctx, 'shard_ctx, T> LocalGuildContext<'ctx, 'shard_ctx, T> {
    /// Create a new [`GuildInteractionContext`] from [interaction context](InteractionContext).
    ///
    /// This function assumes that the interaction given was
    /// invoked from a local guild.
    pub fn from_ctx(ctx: &'ctx InteractionContext<'shard_ctx, T>) -> Result<Self> {
        let member = ctx
            .interaction
            .member
            .as_ref()
            .ok_or_else(|| Error::unknown(NotInLocalGuildError))?;

        let guild_id = ctx
            .interaction
            .guild_id
            .as_ref()
            .ok_or_else(|| Error::unknown(NotInLocalGuildError))?;

        // check if the guild is a local guild
        let local_guild_id = ctx.bot.settings.bot.guild.id;
        if local_guild_id != *guild_id {
            return Err(Error::unknown(NotInLocalGuildError));
        }

        let author = member
            .user
            .as_ref()
            .ok_or_else(|| Error::unknown(NotInLocalGuildError))?;

        Ok(Self {
            author,
            guild_id,
            member,
            inner: &ctx,
        })
    }

    /// This function gets the permissions given to the invoker
    /// from the local guild.
    #[must_use]
    pub fn member_permissions(&self) -> Permissions {
        let cache = self.bot.cache.permissions();
        let member_permissions = cache.root(self.author.id, *self.guild_id).ok();
        if member_permissions.is_none() && self.bot.is_cache_enabled() {
            warn!("could not resolve member permissions of {:?}. using data from InteractionCreate instead", self.author.id);
        }

        member_permissions
            .or(self.member.permissions)
            .unwrap_or_else(Permissions::empty)
    }
}

impl<'ctx, 'shard_ctx, T> Deref for LocalGuildContext<'ctx, 'shard_ctx, T> {
    type Target = InteractionContext<'shard_ctx, T>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

/// Extension of [`InteractionContext`] but it contains guild data
/// where the interaction was invoked.
#[derive(Debug)]
pub struct GuildInteractionContext<'ctx, 'shard_ctx, T> {
    /// User that invoked the interaction.
    pub author: &'ctx User,
    /// ID of the guild where the interaction was invoked.
    pub guild_id: &'ctx Id<GuildMarker>,
    /// Guild member object of the invoker.
    pub member: &'ctx PartialMember,
    /// Inner data of [`GuildInteractionContext`].
    pub inner: &'ctx InteractionContext<'shard_ctx, T>,
}

#[derive(Debug, Error)]
#[error("unexpected interaction was invoked in a non-guild channel")]
pub struct NotInGuildError;

impl<'ctx, 'shard_ctx, T> GuildInteractionContext<'ctx, 'shard_ctx, T> {
    /// Create a new [`GuildInteractionContext`] from [interaction context](InteractionContext).
    ///
    /// This function assumes that the interaction given was
    /// invoked from a guild.
    pub fn from_ctx(ctx: &'ctx InteractionContext<'shard_ctx, T>) -> Result<Self> {
        let member = ctx
            .interaction
            .member
            .as_ref()
            .ok_or_else(|| Error::unknown(NotInGuildError))?;

        let guild_id = ctx
            .interaction
            .guild_id
            .as_ref()
            .ok_or_else(|| Error::unknown(NotInGuildError))?;

        let author = member
            .user
            .as_ref()
            .ok_or_else(|| Error::unknown(NotInGuildError))?;

        Ok(Self {
            author,
            guild_id,
            member,
            inner: &ctx,
        })
    }
}

impl<'ctx, 'shard_ctx, T> Deref for GuildInteractionContext<'ctx, 'shard_ctx, T> {
    type Target = InteractionContext<'shard_ctx, T>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}
