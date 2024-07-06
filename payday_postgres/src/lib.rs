pub mod block_height;
pub mod btc_onchain;

use btc_onchain::OnChainProcessor;
use cqrs_es::{Aggregate, Query};
use payday_btc::{
    on_chain_aggregate::BtcOnChainInvoice,
    on_chain_api::{OnChainApi, OnChainTransactionStreamSubscriber},
};
use payday_core::{persistence::cqrs::Cqrs, PaydayError, PaydayResult};
use postgres_es::{postgres_cqrs, PostgresEventRepository};
use sqlx::{Pool, Postgres};

pub async fn create_postgres_pool(connection_string: &str) -> PaydayResult<Pool<Postgres>> {
    let pool = sqlx::PgPool::connect(connection_string)
        .await
        .map_err(|e| PaydayError::DbError(e.to_string()))?;
    Ok(pool)
}

pub async fn create_cqrs<A>(
    pool: Pool<Postgres>,
    queries: Vec<Box<dyn Query<A>>>,
    services: A::Services,
) -> PaydayResult<Cqrs<A, PostgresEventRepository>>
where
    A: Aggregate,
{
    let cqrs = postgres_cqrs(pool, queries, services);
    Ok(cqrs)
}

pub async fn create_btc_on_chain_processor(
    pool: Pool<Postgres>,
    name: &str,
    on_chain_api: Box<dyn OnChainApi>,
    tx_stream: Box<dyn OnChainTransactionStreamSubscriber>,
) -> PaydayResult<OnChainProcessor> {
    let cqrs = create_cqrs::<BtcOnChainInvoice>(pool, Vec::new(), ()).await?;
    Ok(OnChainProcessor::new(
        name.to_string(),
        "btc_onchain".to_string(),
        on_chain_api,
        tx_stream,
        cqrs,
    ))
}
