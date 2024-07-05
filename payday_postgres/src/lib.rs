pub mod block_height;
use payday_core::{PaydayError, PaydayResult};
use sqlx::{Pool, Postgres};


pub async fn create_postgres_pool(connection_string: &str) -> PaydayResult<Pool<Postgres>> {
    let pool = sqlx::PgPool::connect(connection_string).await.map_err(|e| PaydayError::DbError(e.to_string()))?;
    Ok(pool)
}
