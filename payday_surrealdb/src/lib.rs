use payday_core::{PaydayError, PaydayResult};
use surrealdb::{
    engine::any::{self, Any},
    Error, Surreal,
};

pub mod block_height;

pub async fn create_surreal_db(
    path: &str,
    namespace: &str,
    database: &str,
) -> PaydayResult<Surreal<Any>> {
    let db = any::connect(path)
        .await
        .map_err(|e| PaydayError::DbError(e.to_string()))?;
    db.use_ns(namespace)
        .use_db(database)
        .await
        .map_err(|e| PaydayError::DbError(e.to_string()))?;
    Ok(db)
}
