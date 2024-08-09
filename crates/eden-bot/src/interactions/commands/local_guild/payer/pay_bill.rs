use eden_discord_types::{choices::PaymentMethodOption, commands::local_guild::PayerPayBill};
use eden_utils::Result;
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{CommandContext, RunCommand};
use crate::{
    interactions::{
        record_local_guild_ctx, stateful_commands, LocalGuildContext, StatefulCommandType,
    },
    util::http::request_for_model,
};

const PROMPT_MYNT_MESSAGE: &str = "**To let us know that you're paying with us, please send your {MYNT_ALIAS} screenshot of transfer.**";
const PROMPT_PAYPAL_MESSAGE: &str = "**To let us know that you're paying with us, please send your PayPal screenshot of transfer.**";

impl RunCommand for PayerPayBill {
    #[tracing::instrument(skip(ctx), fields(ctx = tracing::field::Empty))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx).await?;
        record_local_guild_ctx!(ctx);

        // create DM channel
        let dm_channel_id = request_for_model(
            &ctx.bot.http,
            ctx.bot.http.create_private_channel(ctx.author.id),
        )
        .await?
        .id;

        // then, create a message prompting the user to upload or put your reference number and stuff
        let message = match self.method {
            PaymentMethodOption::Mynt => {
                PROMPT_MYNT_MESSAGE.replace("{MYNT_ALIAS}", &*eden_utils::aliases::MYNT_NAME)
            }
            PaymentMethodOption::PayPal => PROMPT_PAYPAL_MESSAGE.to_string(),
        };

        let result = ctx
            .bot
            .http
            .create_message(dm_channel_id)
            .content(&message)
            .unwrap();

        request_for_model(&ctx.bot.http, result).await?;

        let state = StatefulCommandType::PayerPayBill(stateful_commands::PayerPayBill::new(
            dm_channel_id,
            self.method,
        ));
        ctx.bot.command_state.insert(ctx.interaction.id, state);

        let data = InteractionResponseDataBuilder::new()
            .content("**Please proceed to DMs for instructions.**")
            .build();

        ctx.respond(data).await
    }
}
