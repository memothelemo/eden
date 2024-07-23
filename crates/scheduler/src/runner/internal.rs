use crate::{Job, JobSchedule};
use serde_json::Value as Json;

pub struct JobMetadata<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) schedule: ScheduleFn,
}

pub type ScheduleFn = Box<dyn Fn() -> JobSchedule>;
pub type DeserializerFn<State> = Box<
    dyn Fn(
            Json,
        ) -> std::result::Result<
            Box<dyn Job<State = State>>,
            Box<dyn std::error::Error + Send + Sync>,
        > + Send
        + Sync
        + 'static,
>;
