use async_trait::async_trait;
use payday_core::{
    persistence::block_height::{BlockHeight, BlockHeightStoreApi},
    Error, Result,
};
use sqlx::{Pool, Postgres, Row};

pub struct BlockHeightStore {
    db: Pool<Postgres>,
}

impl BlockHeightStore {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    async fn get_block_height_internal(&self, node_id: &str) -> Result<Option<u64>> {
        let res: Option<i64> =
            sqlx::query("SELECT block_height FROM block_height WHERE node_id = $1")
                .bind(node_id)
                .fetch_optional(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?
                .map(|r| r.get("block_height"));
        Ok(res.and_then(|r| u64::try_from(r).ok()))
    }
}

#[async_trait]
impl BlockHeightStoreApi for BlockHeightStore {
    async fn get_block_height(&self, node_id: &str) -> Result<BlockHeight> {
        let height: Option<u64> = self.get_block_height_internal(node_id).await?;
        match height {
            Some(height) => Ok(BlockHeight {
                node_id: node_id.to_string(),
                block_height: height,
            }),
            None => Ok(BlockHeight {
                node_id: node_id.to_string(),
                block_height: 0,
            }),
        }
    }

    async fn set_block_height(&self, node_id: &str, block_height: u64) -> Result<()> {
        let existing: Option<u64> = self.get_block_height_internal(node_id).await?;
        if existing.is_some() {
            sqlx::query("UPDATE block_height SET block_height = $1 WHERE node_id = $2")
                .bind(block_height as i64)
                .bind(node_id)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        } else {
            sqlx::query("INSERT INTO block_height (node_id, block_height) VALUES ($1, $2)")
                .bind(node_id)
                .bind(block_height as i64)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        }

        Ok(())
    }
}
