use eden_discord_types::choices::PaymentMethodOption;
use eden_tasks::prelude::*;
use eden_utils::{
    error::exts::{IntoTypedError, ResultExt},
    twilight::error::TwilightHttpErrorExt,
    types::Sensitive,
    Result,
};
use serde::{Deserialize, Serialize};
use tracing::trace;
use twilight_mention::Mention;
use twilight_model::{
    http::attachment::Attachment,
    id::{
        marker::{ChannelMarker, UserMarker},
        Id,
    },
};

use crate::{util::http::request_for_model, BotRef};

#[derive(Debug, Deserialize, Serialize)]
pub struct AlertPayment {
    pub biller_id: Id<UserMarker>,
    pub biller_dm_channel_id: Id<ChannelMarker>,
    pub payment_method: PaymentMethodOption,
    pub payment_image_url: Sensitive<String>,
    pub payment_image_ext: String,
}

#[async_trait]
impl Task for AlertPayment {
    type State = BotRef;

    #[allow(clippy::unwrap_used)]
    async fn perform(&self, _ctx: &TaskRunContext, bot: Self::State) -> Result<TaskResult> {
        trace!("fetching payment image");

        let bot = bot.get();
        let response = reqwest::get(self.payment_image_url.as_str())
            .await
            .into_typed_error()
            .attach_printable("could not send request to Discord to download image")?;

        let data = response
            .bytes()
            .await
            .into_typed_error()
            .attach_printable("could not download image data")?;

        let filename = format!("payment_for_{}.{}", self.biller_id, self.payment_image_ext);
        let attachments = vec![Attachment::from_bytes(filename, data.into(), 1)];

        trace!("relying payment image to the alert channel");

        let alert_channel_id = bot.settings.bot.local_guild.alert_channel_id;
        let content = format!(
            "**{}'s payment with {:?} as their payment method**",
            self.biller_id.mention(),
            self.payment_method
        );
        let request = bot
            .http
            .create_message(alert_channel_id)
            .attachments(&attachments)
            .unwrap()
            .content(&content)
            .unwrap();

        let result = request_for_model(&bot.http, request)
            .await
            .attach_printable("failed to send message to the alert channel");

        if let Some(info) = result.discord_http_error_info() {
            if info.has_missing_access() {
                let request = bot
                    .http
                    .create_message(self.biller_dm_channel_id)
                    .content(OOPS_MSG)
                    .unwrap();

                request_for_model(&bot.http, request)
                    .await
                    .attach_printable("failed to send error message to the biller")?;

                return Ok(TaskResult::Reject(result.unwrap_err().anonymize()));
            }
        }
        result?;

        Ok(TaskResult::Completed)
    }

    fn kind() -> &'static str {
        "eden::tasks::alert_payment"
    }

    fn priority() -> TaskPriority {
        TaskPriority::High
    }
}

const OOPS_MSG: &str = "**Uhh. It seems like I cannot process payment to the admins. Please report them immediately!**";
