#![feature(let_chains, result_flattening)]

pub mod backoff;
pub mod error;
pub mod task;
pub mod worker;

pub use self::scheduled::Scheduled;
pub use self::settings::Settings;
pub use self::task::{Task, TaskPriority, TaskResult, TaskRunContext, TaskTrigger};
// pub use self::worker::{Worker, WorkerId};

pub mod prelude {
    pub use super::task::{Task, TaskPriority, TaskResult, TaskRunContext, TaskTrigger};

    pub use ::async_trait::async_trait;
    pub use ::chrono::TimeDelta;
    pub use ::serde;
    pub use ::serde::{Deserialize, Serialize};
}

mod registry;
mod scheduled;
mod settings;
