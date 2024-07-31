use async_trait::async_trait;
use eden_db::{forms::InsertAdminForm, schema::Admin};
use eden_tasks::{Task, TaskPerformInfo, TaskResult};
use eden_utils::{
    error::{AnyResultExt, ResultExt},
    Result,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use twilight_model::id::{marker::UserMarker, Id};

use crate::Bot;

#[derive(Debug, Deserialize, Serialize)]
pub struct UpsertLocalGuildAdmins {
    pub entries: Vec<(Id<UserMarker>, String)>,
}

#[derive(Debug, Error)]
#[error("could not upsert local guild administrators")]
struct Error;

#[async_trait]
impl Task for UpsertLocalGuildAdmins {
    type State = Bot;

    async fn perform(&self, _info: &TaskPerformInfo, bot: Self::State) -> Result<TaskResult> {
        let mut conn = bot.db_transaction().await.transform_context(Error)?;
        for (id, name) in self.entries.iter() {
            let form = InsertAdminForm::builder().id(*id).name(Some(&name)).build();
            Admin::upsert(&mut conn, form)
                .await
                .change_context(Error)
                .attach_printable_lazy(|| {
                    format!("could not upsert for local guild admin ({id})")
                })?;
        }

        conn.commit()
            .await
            .change_context(Error)
            .attach_printable("could not commit transaction")?;

        Ok(TaskResult::Completed)
    }

    fn task_type() -> &'static str
    where
        Self: Sized,
    {
        "admins::upsert::from_local_guild"
    }

    fn temporary() -> bool {
        true
    }
}
