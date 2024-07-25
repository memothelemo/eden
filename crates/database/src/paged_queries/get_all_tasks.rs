use sqlx::postgres::PgArguments;
use sqlx::Arguments;

use crate::schema::{Task, TaskStatus};
use crate::utils::{PagedQuery, Paginated};

#[must_use]
pub struct GetAllTasks {
    status: Option<TaskStatus>,
}

impl GetAllTasks {
    pub fn new() -> Self {
        Self { status: None }
    }

    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn build(self) -> Paginated<Self> {
        Paginated::new(self)
    }
}

impl PagedQuery for GetAllTasks {
    type Output = Task;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        if let Some(status) = self.status.as_ref() {
            args.add(status);
        }
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SELECT * FROM tasks ")?;
        if self.status.is_some() {
            f.write_str("WHERE status = $1 ")?;
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
