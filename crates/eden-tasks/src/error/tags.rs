use eden_tasks_schema::types::TaskStatus;
use eden_utils::Error;
use serde::{ser::SerializeMap, Serialize};
use uuid::Uuid;

pub fn install_hook() {
    DeleteTaskTag::install_hook();
    ClearAllWithStatusTag::install_hook();
    ScheduleTaskTag::install_hook();
}

pub struct ScheduleTaskTag {
    kind: String,
    data: String,
}

impl ScheduleTaskTag {
    pub(crate) fn new<S: Clone + Send + Sync + 'static, T>(data: &T) -> Self
    where
        T: crate::Task<State = S>,
    {
        Self {
            kind: T::kind().into(),
            data: format!("{data:?}"),
        }
    }

    fn install_hook() {
        Error::install_serde_hook::<Self>();
        Error::install_hook::<Self>(|this, ctx| {
            ctx.push_body(format!("task.type: {:?}", this.kind));
            ctx.push_body(format!("task.data: {}", this.data));
        });
    }
}

impl Serialize for ScheduleTaskTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // this is to differentiate various attachments
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("_type", "SCHEDULE_TASK_TAG")?;
        map.serialize_entry("task.type", &self.kind)?;
        map.serialize_entry("task.data", &self.data)?;
        map.end()
    }
}

#[derive(Clone, Copy)]
pub struct DeleteTaskTag {
    pub(crate) id: Uuid,
}

impl DeleteTaskTag {
    fn install_hook() {
        Error::install_serde_hook::<Self>();
        Error::install_hook::<Self>(|this, ctx| {
            ctx.push_body(format!("with id: {:?}", this.id));
        });
    }
}

impl Serialize for DeleteTaskTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // this is to differentiate various attachments
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("_type", "DELETE_TASK_TAG")?;
        map.serialize_entry("task.id", &self.id)?;
        map.end()
    }
}

#[derive(Clone, Copy)]
pub struct ClearAllWithStatusTag {
    pub(crate) status: Option<TaskStatus>,
    pub(crate) task: Option<(&'static str, &'static str)>,
}

impl ClearAllWithStatusTag {
    pub(crate) fn none() -> Self {
        Self {
            task: None,
            status: None,
        }
    }

    pub(crate) fn task(kind: &'static str, rust_name: &'static str) -> Self {
        Self {
            task: Some((kind, rust_name)),
            status: None,
        }
    }

    pub(crate) fn status(status: TaskStatus) -> Self {
        Self {
            task: None,
            status: Some(status),
        }
    }

    fn install_hook() {
        Error::install_serde_hook::<Self>();
        Error::install_hook::<Self>(|this, ctx| {
            if let Some((kind, rust_name)) = this.task {
                ctx.push_body(format!("with task type: {kind:?} ({rust_name})"));
            }
            ctx.push_body(format!("with status: {:?}", this.status));
        });
    }
}

impl Serialize for ClearAllWithStatusTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // this is to differentiate various attachments
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("_type", "CLEAR_ALL_TASKS_WITH_STATUS_TAG")?;
        map.serialize_entry("filter", &self.task)?;
        map.serialize_entry("status", &self.status)?;
        map.end()
    }
}
