use eden_utils::sql::{PageQueyer, Paginated};
use sqlx::postgres::PgArguments;
use sqlx::Arguments;

use crate::types::{Task, TaskStatus, WorkerId};

#[must_use]
pub struct GetAllTasks<'a> {
    periodic: Option<bool>,
    status: Option<TaskStatus>,
    task_type: Option<&'a str>,
    worker_id: WorkerId,
}

impl<'a> GetAllTasks<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new(worker_id: WorkerId) -> Self {
        Self {
            periodic: None,
            status: None,
            task_type: None,
            worker_id,
        }
    }

    pub fn periodic(mut self, periodic: bool) -> Self {
        self.periodic = Some(periodic);
        self
    }

    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn task_type(mut self, task_type: &'a str) -> Self {
        self.task_type = Some(task_type);
        self
    }

    pub fn build(self) -> Paginated<Self> {
        Paginated::new(self)
    }
}

impl<'a> PageQueyer for GetAllTasks<'a> {
    type Output = Task;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        if let Some(status) = self.status.as_ref() {
            args.add(status);
        }
        if let Some(task_type) = self.task_type.as_ref() {
            args.add(task_type);
        }
        if let Some(periodic) = self.periodic {
            args.add(periodic);
        }
        args.add(self.worker_id.assigned_sql());
        args.add(self.worker_id.total_sql());
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SELECT * FROM tasks ")?;

        let mut count = 0;
        if self.status.is_some() {
            write!(f, "WHERE ")?;
            count += 1;
            write!(f, "status = ${count} ")?;
        }
        if self.task_type.is_some() {
            if count == 0 {
                write!(f, "WHERE ")?;
            } else {
                write!(f, " AND ")?;
            }
            count += 1;
            write!(f, "data->>'type' = ${count} ")?;
        }
        if self.periodic.is_some() {
            if count == 0 {
                write!(f, "WHERE ")?;
            } else {
                write!(f, " AND ")?;
            }
            count += 1;
            write!(f, "periodic = ${count} ")?;
        }

        if count == 0 {
            write!(f, "WHERE ")?;
        } else {
            write!(f, " AND ")?;
        }
        count += 1;

        let worker_id_count = count;
        count += 1;

        let total_workers_count = count;
        write!(
            f,
            r#"get_worker_id_from_task(task_number, ${total_workers_count}) = ${worker_id_count} "#
        )?;
        f.write_str("FOR UPDATE SKIP LOCKED")
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils;
    use eden_utils::{error::exts::AnonymizeErrorInto, sql::Paginated};

    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_periodic_filter(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Paginated::new(GetAllTasks::new(WorkerId::ONE).periodic(true)).size(3);
        while let Some(data) = stream.next(&mut conn).await? {
            assert!(data.iter().all(|v| v.periodic));
        }

        let mut stream = Paginated::new(GetAllTasks::new(WorkerId::ONE).periodic(false)).size(3);
        while let Some(data) = stream.next(&mut conn).await? {
            assert!(data.iter().all(|v| !v.periodic));
        }

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_task_type_filter(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Paginated::new(GetAllTasks::new(WorkerId::ONE).task_type("foo")).size(3);
        while let Some(data) = stream.next(&mut conn).await? {
            assert!(data.iter().all(|v| v.data.kind == "foo"));
        }

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Paginated::new(GetAllTasks::new(WorkerId::ONE)).size(3);
        while let Some(data) = stream.next(&mut conn).await? {
            assert_eq!(data.len(), 3);
        }

        Ok(())
    }
}
