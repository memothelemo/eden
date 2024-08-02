use sqlx::{postgres::PgArguments, QueryBuilder, Row};
use std::result::Result as StdResult;

use super::error::QueryError;
use crate::error::exts::{IntoResult, ResultExtInto};
use crate::error::Result;

#[must_use]
pub struct Paginated<Q> {
    page: i64,
    offset: Option<i64>,
    size: i64,

    prerun: bool,
    queryer: Q,
}

#[allow(async_fn_in_trait)]
pub trait PageQueyer {
    type Output: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Sync + Unpin;

    fn build_args(&self) -> PgArguments {
        PgArguments::default()
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    /// This function runs before the actual [pagination](Paginated) happens.
    #[allow(unused)]
    async fn prerun(&self, conn: &mut sqlx::PgConnection) -> Result<(), QueryError> {
        Ok(())
    }
}

impl<Q: PageQueyer> Paginated<Q> {
    const DEFAULT_SIZE: i64 = 10;

    #[must_use]
    pub fn new(queryer: Q) -> Self {
        Self {
            page: 0,
            offset: Some(0),
            size: Self::DEFAULT_SIZE,
            prerun: false,
            queryer,
        }
    }

    pub async fn next(
        &mut self,
        conn: &mut sqlx::PgConnection,
    ) -> Result<Option<Vec<Q::Output>>, QueryError> {
        if !self.prerun {
            self.queryer
                .prerun(conn)
                .await
                .attach_printable("could not perform query prerun before pagination")?;

            self.prerun = true;
        }

        // Don't try go to the next page if offset is None
        let Some(offset) = self.offset.as_mut() else {
            return Ok(None);
        };

        let mut builder = QueryBuilder::<sqlx::Postgres>::with_arguments(
            r#"SELECT *, COUNT(*) OVER () AS "__total__" FROM ("#,
            self.queryer.build_args(),
        );
        let offset = *offset;
        self.page += 1;

        // SAFETY:
        // SQL injection is not a possibility since the input for the query parameter
        // depends on how it is being used from the programmer unless if it is
        // configured it incorrectly.
        builder.push(self.generate_sql());
        builder.push(r") t LIMIT ");
        builder.push_bind(self.size);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        let query = builder.build_query_as::<PaginationResult<Q::Output>>();
        let results = query
            .fetch_all(conn)
            .await
            .change_context_into(QueryError)
            .attach_printable("could not paginate entries")?;

        let overall_total = results.first().map_or(0, |x| x.overall_total);
        let records: Vec<Q::Output> = results.into_iter().map(|x| x.data).collect();

        // Does it exceeds the predicted amount of entries per page for the next page?
        let total_pages_read = offset + self.size;
        self.offset = if overall_total > total_pages_read {
            Some(calculate_offset(self.page + 1, self.size))
        } else {
            None
        };

        Ok(if records.is_empty() {
            None
        } else {
            Some(records)
        })
    }

    #[must_use]
    pub fn current_page(&self) -> i64 {
        self.page
    }

    fn generate_sql(&self) -> String {
        struct SqlRenderer<'a, T>(&'a T);

        impl<'a, T> std::fmt::Display for SqlRenderer<'a, T>
        where
            T: PageQueyer,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.build_sql(f)
            }
        }

        SqlRenderer(&self.queryer).to_string()
    }
}

struct PaginationResult<T> {
    data: T,
    overall_total: i64,
}

impl<'r, T> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PaginationResult<T>
where
    T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    fn from_row(row: &'r sqlx::postgres::PgRow) -> StdResult<Self, sqlx::Error> {
        let data = T::from_row(row)?;
        let overall_total = row.try_get("__total__")?;
        Ok(Self {
            data,
            overall_total,
        })
    }
}

const fn calculate_offset(page: i64, size: i64) -> i64 {
    (match page.checked_sub(1) {
        Some(n) => n,
        None => 0,
    }) * size
}
