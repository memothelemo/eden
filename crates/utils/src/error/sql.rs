use sqlx::error::ErrorKind;

pub trait SqlErrorExt {
    fn has_unique_violation(&self) -> bool;
}

impl<T, C> SqlErrorExt for crate::Result<T, C> {
    fn has_unique_violation(&self) -> bool {
        match self {
            Err(error) => error.has_unique_violation(),
            Ok(..) => false,
        }
    }
}

impl<T> SqlErrorExt for crate::Error<T> {
    fn has_unique_violation(&self) -> bool {
        // Get the SQLx error data
        let Some(sqlx::Error::Database(error)) = self.report.downcast_ref::<sqlx::Error>() else {
            return false;
        };
        matches!(error.kind(), ErrorKind::UniqueViolation)
    }
}
