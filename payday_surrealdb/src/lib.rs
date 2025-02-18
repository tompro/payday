use chrono::Utc;
use payday_core::{Error, Result};
use serde::{Serialize, Serializer};
use surrealdb::{
    engine::any::{self, Any},
    Surreal,
};

pub mod block_height;
pub mod event_stream;
pub mod task;

pub async fn create_surreal_db(
    path: &str,
    namespace: &str,
    database: &str,
) -> Result<Surreal<Any>> {
    let db = any::connect(path)
        .await
        .map_err(|e| Error::DbError(e.to_string()))?;
    db.use_ns(namespace)
        .use_db(database)
        .await
        .map_err(|e| Error::DbError(e.to_string()))?;
    Ok(db)
}

pub fn serialize_chrono_as_sql_datetime<S>(
    x: &chrono::DateTime<Utc>,
    s: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    Into::<surrealdb::sql::Datetime>::into(*x).serialize(s)
}

pub fn serialize_chrono_as_sql_datetime_option<S>(
    x: &Option<chrono::DateTime<Utc>>,
    s: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(x) => Into::<surrealdb::sql::Datetime>::into(*x).serialize(s),
        None => s.serialize_none(),
    }
}
