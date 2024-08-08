use eden_tasks::prelude::*;
use eden_utils::error::exts::*;
use eden_utils::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::errors::SetupLocalGuildError;
use crate::BotRef;

#[derive(Debug, Deserialize, Serialize)]
pub struct SetupLocalGuild;

#[async_trait]
impl Task for SetupLocalGuild {
    type State = BotRef;

    async fn perform(&self, _ctx: &TaskRunContext, state: Self::State) -> Result<TaskResult> {
        let bot = state.get();
        let local_guild_id = bot.settings.bot.local_guild.id;

        debug!("fetching guild information for local guild {local_guild_id}");
        let guild = crate::util::http::request_for_model(&bot.http, bot.http.guild(local_guild_id))
            .await
            .change_context(SetupLocalGuildError)
            .attach_printable_lazy(|| format!("could not request guild data for {local_guild_id}"))
            .attach(crate::suggestions::NO_LOCAL_GUILD)?;

        crate::local_guild::setup(&bot, &guild).await?;
        Ok(TaskResult::Completed)
    }

    fn kind() -> &'static str {
        "eden::tasks::setup_local_guild"
    }

    fn priority() -> TaskPriority {
        TaskPriority::High
    }

    fn temporary() -> bool {
        true
    }
}
