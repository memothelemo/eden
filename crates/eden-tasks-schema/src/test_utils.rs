use chrono::{TimeDelta, Utc};
use eden_utils::error::exts::ResultExt;
use eden_utils::Result;

use crate::forms::InsertTaskForm;
use crate::types::{Task, TaskPriority, TaskRawData};

pub async fn generate_task(conn: &mut sqlx::PgConnection) -> Result<Task> {
    let form = InsertTaskForm::builder()
        .deadline(Utc::now())
        .priority(TaskPriority::default())
        .data(TaskRawData {
            kind: "foo".into(),
            inner: serde_json::json!({
                "currency": "PHP",
                "deadline": Utc::now(),
                "payer_id": "613425648685547541",
                "price": 15.0,
            }),
        })
        .build();

    Task::insert(conn, form).await.anonymize_error()
}

pub async fn prepare_sample_tasks(conn: &mut sqlx::PgConnection) -> eden_utils::Result<()> {
    // prepare 5 sample deadlines
    let deadline_1 = Utc::now();
    let deadline_2 = deadline_1
        .checked_add_signed(TimeDelta::seconds(5))
        .unwrap();

    let deadline_3 = deadline_2
        .checked_add_signed(TimeDelta::seconds(3))
        .unwrap();

    let deadline_4 = deadline_3
        .checked_add_signed(TimeDelta::seconds(1))
        .unwrap();

    let deadline_5 = deadline_4
        .checked_add_signed(TimeDelta::milliseconds(500))
        .unwrap();

    // Then prepare these tasks for some reason :)
    let task = serde_json::json!({
        "currency": "PHP",
        "deadline": Utc::now(),
        "payer_id": "613425648685547541",
        "price": 15.0,
    });

    // Prepare a list of tasks (situation stuff)
    // - deadline_1 - high priority
    // - deadline_2 - low priority
    // - deadline_1 - medium priority
    // - deadline_3 - high priority and so on
    macro_rules! shorthand_insert {
        ($deadline:ident, $priority:ident, $kind:literal, $periodic:expr) => {{
            Task::insert(
                conn,
                InsertTaskForm::builder()
                    .deadline($deadline)
                    .priority(TaskPriority::$priority)
                    .data(TaskRawData {
                        kind: $kind.into(),
                        inner: task.clone(),
                    })
                    .build(),
            )
            .await
            .anonymize_error()?;
        }};
    }

    shorthand_insert!(deadline_1, High, "organ", true);
    shorthand_insert!(deadline_3, Low, "organ", true);
    shorthand_insert!(deadline_4, High, "organ", false);
    shorthand_insert!(deadline_1, Low, "organ", false);
    shorthand_insert!(deadline_5, High, "organ", true);
    shorthand_insert!(deadline_2, Low, "organ", true);
    shorthand_insert!(deadline_5, Medium, "organ", false);
    shorthand_insert!(deadline_1, Medium, "organ", false);
    shorthand_insert!(deadline_3, High, "foo", true);
    shorthand_insert!(deadline_5, Low, "foo", true);
    shorthand_insert!(deadline_2, High, "foo", false);
    shorthand_insert!(deadline_4, Medium, "foo", false);
    shorthand_insert!(deadline_2, Medium, "foo", true);
    shorthand_insert!(deadline_3, Medium, "foo", true);
    shorthand_insert!(deadline_4, Low, "foo", false);

    Ok(())
}
