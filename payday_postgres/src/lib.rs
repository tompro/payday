pub mod btc_onchain;
pub mod offset;

use cqrs_es::{persist::PersistedEventStore, Aggregate, CqrsFramework, Query};
use payday_core::{persistence::cqrs::Cqrs, Error, Result};
use postgres_es::PostgresEventRepository;
use sqlx::{Pool, Postgres};

pub async fn create_postgres_pool(connection_string: &str) -> Result<Pool<Postgres>> {
    let pool = sqlx::PgPool::connect(connection_string)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
    Ok(pool)
}

pub async fn init_tables(pool: Pool<Postgres>) -> Result<()> {
    let sql = include_str!("../db/migrations/init.sql");
    sqlx::raw_sql(sql)
        .execute(&pool)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
    Ok(())
}

pub async fn create_cqrs<A>(
    pool: Pool<Postgres>,
    queries: Vec<Box<dyn Query<A>>>,
    services: A::Services,
) -> Result<Cqrs<A, PostgresEventRepository>>
where
    A: Aggregate,
{
    //let cqrs = postgres_cqrs(pool, queries, services);
    let repo = PostgresEventRepository::new(pool).with_tables("payday.events", "payday.snapshots");
    let store = PersistedEventStore::new_event_store(repo);
    Ok(CqrsFramework::new(store, queries, services))
}

#[cfg(test)]
mod test_utils {
    use sqlx::{Pool, Postgres};
    use testcontainers::ContainerAsync;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use tokio::sync::OnceCell;

    static POSTGRES_CONTAINER: OnceCell<
        ContainerAsync<testcontainers_modules::postgres::Postgres>,
    > = OnceCell::const_new();

    async fn get_postgres_container(
    ) -> &'static ContainerAsync<testcontainers_modules::postgres::Postgres> {
        POSTGRES_CONTAINER
            .get_or_init(|| async {
                testcontainers_modules::postgres::Postgres::default()
                    .start()
                    .await
                    .expect("unable to start postgres container")
            })
            .await
    }

    static POSTGRES_POOL: OnceCell<Pool<Postgres>> = OnceCell::const_new();

    /// crates a static postgres that will be the same for all tests. So when testing
    /// keep in mind that the database might have state from other tests.
    pub async fn get_postgres_pool() -> Pool<Postgres> {
        let pool = POSTGRES_POOL
            .get_or_init(|| async {
                let container = get_postgres_container().await;
                let connection_string = format!(
                    "postgres://postgres:postgres@127.0.0.1:{}/postgres",
                    container
                        .get_host_port_ipv4(5432)
                        .await
                        .expect("unable to get postgres test port")
                );
                let pool = super::create_postgres_pool(&connection_string)
                    .await
                    .expect("unable to create postgres pool");
                super::init_tables(pool.clone())
                    .await
                    .expect("unable to init tables");
                pool
            })
            .await;
        pool.clone()
    }
}
