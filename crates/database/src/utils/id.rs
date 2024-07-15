use std::fmt::{Debug, Display};
use thiserror::Error;
use twilight_model::id::Id;

#[repr(transparent)]
pub struct SqlSnowflake<T>(Id<T>);

impl<T> SqlSnowflake<T> {
    #[must_use]
    pub const fn new(value: Id<T>) -> Self {
        Self(value)
    }
}

impl<T> Debug for SqlSnowflake<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T> Display for SqlSnowflake<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T> From<Id<T>> for SqlSnowflake<T> {
    #[inline]
    fn from(value: Id<T>) -> Self {
        Self(value)
    }
}

impl<T> From<SqlSnowflake<T>> for Id<T> {
    #[inline]
    fn from(value: SqlSnowflake<T>) -> Self {
        value.0
    }
}

#[derive(Debug, Error)]
#[error("unexpected snowflake id to be a value of {0:?}")]
struct InvalidId(i64);

impl<'row, T> sqlx::Decode<'row, sqlx::Postgres> for SqlSnowflake<T>
where
    i64: sqlx::Decode<'row, sqlx::Postgres>,
{
    // Discord uses up to 63 bits in 64-bit signed integer for
    // their snowflake ID anyways. No sign loss or unexpected
    // output will happen. :)
    //
    // Reference: https://discord.com/developers/docs/reference#snowflakes-snowflake-id-format-structure-left-to-right
    #[allow(clippy::cast_sign_loss)]
    fn decode(value: sqlx::postgres::PgValueRef<'row>) -> Result<Self, sqlx::error::BoxDynError> {
        // Make sure the value is not negative nor zero, this is very important
        // for 64 bit unsigned non-zero integers.
        let value = i64::decode(value)?;
        if value.is_negative() || value == 0 {
            return Err(Box::new(InvalidId(value)));
        }

        if let Some(id) = Id::new_checked(value as u64) {
            Ok(SqlSnowflake(id))
        } else {
            Ok(SqlSnowflake(Id::new(1)))
        }
    }
}

fn exceeds_63_bits(value: u64) -> bool {
    value >> 63 == 1
}

// Twilight does not validate if there's an exceeding bit/s beyond 63 bits of snowflake
// data as referenced to the Discord's snowflake ID structure, we need to check if we have
// ONLY 63 BITS inside SqlSnowflake type.
impl<'query, T> sqlx::Encode<'query, sqlx::Postgres> for SqlSnowflake<T>
where
    i64: sqlx::Encode<'query, sqlx::Postgres>,
{
    #[allow(clippy::cast_possible_wrap)]
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::database::HasArguments<'query>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        let value = self.0.get();
        if exceeds_63_bits(value) {
            // There's no way to know what is the actual cause of the query error by
            // hiding the fact that the value of the id actually exceeds the max. signed
            // integer value, so we really need to log this.
            tracing::error!(
                ?value,
                "Snowflake id is out of bounds of the max signed integer value"
            );
            return sqlx::encode::IsNull::Yes;
        }

        // We already checked it with exceeds_63_bits means signed
        // integers can be negative by setting it to 1 at the 64th bit.
        let value = value as i64;
        value.encode(buf)
    }
}

impl<T> sqlx::Type<sqlx::Postgres> for SqlSnowflake<T> {
    fn compatible(ty: &<sqlx::Postgres as sqlx::Database>::TypeInfo) -> bool {
        <i64 as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }

    fn type_info() -> <sqlx::Postgres as sqlx::Database>::TypeInfo {
        <i64 as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

#[cfg(test)]
mod tests {
    use super::exceeds_63_bits;

    #[test]
    fn test_exceeds_63_bits() {
        assert!(!exceeds_63_bits(10));
        assert!(exceeds_63_bits(u64::MAX));
    }
}
