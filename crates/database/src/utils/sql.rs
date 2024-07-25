use eden_utils::error::{AnyResultExt, ResultExt};
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::{FromRow, Postgres, QueryBuilder, Row};

use crate::QueryError;

// TODO: explain about pagination
#[must_use = "Paginated is a lazy object, use '.next()' to get a page of records"]
pub struct Paginated<T> {
    ran_prerun: bool,
    page: i64,
    size: i64,
    offset: Option<i64>,
    builder: T,
}

struct BuildSql<'a, T>(&'a T);

impl<'a, T> std::fmt::Display for BuildSql<'a, T>
where
    T: PagedQuery,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.build_sql(f)
    }
}

impl<T> Paginated<T>
where
    T: PagedQuery,
{
    const DEFAULT_SIZE_PER_PAGE: i64 = 10;

    pub(crate) fn new(builder: T) -> Self {
        Self {
            ran_prerun: false,
            page: 0,
            size: Self::DEFAULT_SIZE_PER_PAGE,
            offset: Some(0),
            builder,
        }
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn size(mut self, size: u64) -> Self {
        // reset everything
        self.ran_prerun = false;
        self.size = (size as i64).abs();
        self.page = 0;
        self.offset = Some(0);
        self
    }

    pub async fn next<'args>(
        &mut self,
        conn: &mut sqlx::PgConnection,
    ) -> eden_utils::Result<Option<Vec<T::Output>>, QueryError> {
        // Prerun first...
        if !self.ran_prerun {
            self.builder
                .prerun(conn)
                .await
                .attach_printable("could not perform prerun")?;

            self.ran_prerun = true;
        }

        // Don't try go to the next page if offset is None
        let Some(offset) = self.offset.as_mut() else {
            return Ok(None);
        };

        let mut builder = QueryBuilder::<Postgres>::with_arguments(
            r#"SELECT *, COUNT(*) OVER () AS "__total__" FROM ("#,
            self.builder.build_args(),
        );

        self.page += 1;

        // SAFETY:
        // SQL injection is not a possibility since the input for the query parameter
        // depends on how it is being used from the programmer unless if it is
        // configured it incorrectly.
        let generated_sql = BuildSql(&self.builder).to_string();
        builder.push(generated_sql);
        builder.push(r") t LIMIT ");
        builder.push_bind(self.size);
        builder.push(" OFFSET ");
        builder.push_bind(*offset);

        let query = builder.build_query_as::<PaginationResult<T::Output>>();
        let results = query
            .fetch_all(conn)
            .await
            .anonymize_error()
            .transform_context(QueryError)
            .attach_printable("could not paginate entries")?;

        let overall_total = results.first().map_or(0, |x| x.overall_total);
        let records: Vec<T::Output> = results.into_iter().map(|x| x.data).collect();

        // Does it exceeds the predicted amount of entries per page for the next page?
        let total_pages_read = *offset + self.size;
        self.offset = if overall_total > total_pages_read {
            Some(calculate_offset(self.page + 1, self.size))
        } else {
            None
        };

        // expected to have some records
        Ok(if records.is_empty() {
            None
        } else {
            Some(records)
        })
    }
}

pub trait PagedQuery {
    type Output: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin;

    fn build_args(&self) -> PgArguments {
        PgArguments::default()
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    fn prerun(
        &self,
        _conn: &mut sqlx::PgConnection,
    ) -> impl std::future::Future<Output = eden_utils::Result<(), QueryError>> + Send {
        futures::future::ok(())
    }
}

struct PaginationResult<T> {
    data: T,
    overall_total: i64,
}

impl<'r, T> sqlx::FromRow<'r, PgRow> for PaginationResult<T>
where
    T: sqlx::FromRow<'r, PgRow> + Send + Unpin,
{
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
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

impl<T> Paginated<T>
where
    T: PagedQuery,
{
    #[must_use]
    pub fn current_page(&self) -> i64 {
        self.page
    }
}

#[derive(Debug)]
pub(crate) struct CountResult {
    pub total: i64,
}

impl<'r> FromRow<'r, PgRow> for CountResult {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            total: row.try_get("total")?,
        })
    }
}
