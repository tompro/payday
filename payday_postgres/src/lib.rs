pub mod block_height;
pub mod btc_onchain;

use cqrs_es::{Aggregate, Query};
use payday_core::{persistence::cqrs::Cqrs, Error, Result};
use postgres_es::{postgres_cqrs, PostgresEventRepository};
use sqlx::{Pool, Postgres};

pub async fn create_postgres_pool(connection_string: &str) -> Result<Pool<Postgres>> {
    let pool = sqlx::PgPool::connect(connection_string)
        .await
        .map_err(|e| Error::DbError(e.to_string()))?;
    Ok(pool)
}

pub async fn create_cqrs<A>(
    pool: Pool<Postgres>,
    queries: Vec<Box<dyn Query<A>>>,
    services: A::Services,
) -> Result<Cqrs<A, PostgresEventRepository>>
where
    A: Aggregate,
{
    let cqrs = postgres_cqrs(pool, queries, services);
    Ok(cqrs)
}
