use crate::interactions::{StatefulCommand, StatefulCommandResult, StatefulCommandTrigger};
use crate::util::http::request_for_model;
use crate::{tasks, Bot};
use eden_discord_types::choices::PaymentMethodOption;
use eden_tasks::Scheduled;
use eden_utils::Result;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;
use tracing::warn;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_model::id::Id;

#[derive(Debug)]
pub struct PayerPayBill {
    pub busy: AtomicBool,
    pub dm_channel_id: Id<ChannelMarker>,
    pub method: PaymentMethodOption,
    pub last_user_message_id: Mutex<Option<Id<MessageMarker>>>,
}

impl PayerPayBill {
    #[must_use]
    pub fn new(dm_channel_id: Id<ChannelMarker>, method: PaymentMethodOption) -> Self {
        Self {
            busy: AtomicBool::new(false),
            dm_channel_id,
            method,
            last_user_message_id: Mutex::new(None),
        }
    }
}

#[allow(clippy::unwrap_used)]
impl StatefulCommand for PayerPayBill {
    #[tracing::instrument(skip(self, bot))]
    async fn on_trigger(
        &self,
        bot: &Bot,
        trigger: StatefulCommandTrigger,
    ) -> Result<StatefulCommandResult> {
        let message_id = match trigger {
            StatefulCommandTrigger::SentMessage(channel_id, message_id)
                if channel_id == self.dm_channel_id =>
            {
                message_id
            }
            _ => return Ok(StatefulCommandResult::Ignore),
        };
        *self.last_user_message_id.lock().await = Some(message_id);

        // Read the message perhaps :)
        let request = bot.http.message(self.dm_channel_id, message_id);
        let result = request_for_model(&bot.http, request).await;
        let message = match result {
            Ok(n) => n,
            Err(error) => {
                let error = error.anonymize();
                warn!(%error, "unable to get message data from Discord");
                self.reply_message(bot, UNABLE_TO_READ_MSG).await?;
                return Ok(StatefulCommandResult::Continue);
            }
        };

        // Make sure the user uploaded an attachment containing JPEG or PNG.
        let Some(attachment) = message.attachments.first() else {
            self.reply_message(bot, NOT_ATTACHMENT_ERROR).await?;
            return Ok(StatefulCommandResult::Continue);
        };

        if !matches!(
            attachment.content_type.as_deref(),
            Some("image/jpeg" | "image/png")
        ) {
            self.reply_message(bot, NOT_ATTACHMENT_ERROR).await?;
            return Ok(StatefulCommandResult::Continue);
        }

        let filename = PathBuf::from(&attachment.filename);
        let file_extension = filename
            .extension()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_default();
        let user_id = message.author.id;

        let task = tasks::AlertPayment {
            biller_id: user_id,
            biller_dm_channel_id: self.dm_channel_id,
            payment_method: self.method,
            payment_image_url: attachment.url.clone().into(),
            payment_image_ext: file_extension,
        };

        if let Err(error) = bot.queue.schedule(task, Scheduled::now()).await {
            let error = error.anonymize();
            warn!(%error, "failed to schedule to alert payments to the admins");

            self.reply_message(bot, UNABLE_TO_READ_MSG).await?;
            return Ok(StatefulCommandResult::Continue);
        }

        // If it does send, relay it to the alert channel
        self.reply_message(bot, SUCCESS).await?;
        Ok(StatefulCommandResult::Done)
    }

    #[tracing::instrument(skip(self, bot))]
    async fn on_inactive(&self, bot: &Bot) -> Result<()> {
        let request = bot
            .http
            .create_message(self.dm_channel_id)
            .content(CANCELLED_PAYMENT_MSG)
            .unwrap();

        request_for_model(&bot.http, request).await?;
        Ok(())
    }
}

impl PayerPayBill {
    #[tracing::instrument(skip_all)]
    async fn reply_message(&self, bot: &Bot, message: &str) -> Result<()> {
        let last_user_message_id = *self.last_user_message_id.lock().await;

        let mut request = bot
            .http
            .create_message(self.dm_channel_id)
            .content(message)
            .unwrap();

        if let Some(last_user_message_id) = last_user_message_id {
            request = request.reply(last_user_message_id);
        }

        request_for_model(&bot.http, request).await?;
        Ok(())
    }
}

const SUCCESS: &str = "**All right!** Thank you for paying your bills and your bill will be sent to the administrators!";
const NOT_ATTACHMENT_ERROR: &str = "**Please upload the proof of transfer image only!**";
const UNABLE_TO_READ_MSG: &str =
    "Sorry. I cannot get your message data. Please try to send it to me again.";

const CANCELLED_PAYMENT_MSG: &str = "**Cancelled payment process because of inactivity. Please try running `/payer pay_bill` again in the server.**";
