use sqlx::postgres::PgArguments;
use sqlx::Arguments;

use crate::schema::{Task, TaskStatus};
use crate::utils::{PagedQuery, Paginated};

#[must_use]
pub struct GetAllTasks<'a> {
    periodic: Option<bool>,
    status: Option<TaskStatus>,
    task_type: Option<&'a str>,
}

impl<'a> GetAllTasks<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            periodic: None,
            status: None,
            task_type: None,
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

impl<'a> PagedQuery for GetAllTasks<'a> {
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
            }
            count += 1;
            write!(f, "data->>'type' = ${count} ")?;
        }
        if self.periodic.is_some() {
            if count == 0 {
                write!(f, "WHERE ")?;
            }
            count += 1;
            write!(f, "periodic = ${count} ")?;
        }
        f.write_str("FOR UPDATE SKIP LOCKED")
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_utils, utils::Paginated};
    use eden_utils::error::ResultExt;

    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_periodic_filter(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Paginated::new(GetAllTasks::new().periodic(true)).size(3);
        while let Some(data) = stream.next(&mut conn).await.anonymize_error()? {
            assert!(data.iter().all(|v| v.periodic == true));
        }

        let mut stream = Paginated::new(GetAllTasks::new().periodic(false)).size(3);
        while let Some(data) = stream.next(&mut conn).await.anonymize_error()? {
            assert!(data.iter().all(|v| v.periodic == false));
        }

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_task_type_filter(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Paginated::new(GetAllTasks::new().task_type("foo")).size(3);
        while let Some(data) = stream.next(&mut conn).await.anonymize_error()? {
            assert!(data.iter().all(|v| v.data.kind == "foo"));
        }

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Paginated::new(GetAllTasks::new()).size(3);
        while let Some(data) = stream.next(&mut conn).await.anonymize_error()? {
            assert_eq!(data.len(), 3);
        }

        Ok(())
    }
}
