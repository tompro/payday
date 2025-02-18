use async_trait::async_trait;
use payday_core::{
    persistence::block_height::{BlockHeight, BlockHeightStoreApi},
    Error, Result,
};
use surrealdb::{engine::any::Any, Surreal};

pub struct BlockHeightStore {
    db: Surreal<Any>,
}

impl BlockHeightStore {
    pub fn new(db: Surreal<Any>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BlockHeightStoreApi for BlockHeightStore {
    async fn get_block_height(&self, node_id: &str) -> Result<BlockHeight> {
        let height: Option<BlockHeight> = self
            .db
            .select(("block_height", node_id))
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;
        match height {
            Some(height) => Ok(height),
            None => Ok(BlockHeight {
                node_id: node_id.to_string(),
                block_height: 0,
            }),
        }
    }

    async fn set_block_height(&self, node_id: &str, block_height: u64) -> Result<()> {
        let data = BlockHeight {
            node_id: node_id.to_string(),
            block_height,
        };
        let existing: Option<BlockHeight> = self
            .db
            .select(("block_height", node_id))
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        if existing.is_some() {
            let _: Option<BlockHeight> = self
                .db
                .update(("block_height", node_id))
                .content(data)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        } else {
            let _: Option<BlockHeight> = self
                .db
                .create(("block_height", node_id))
                .content(data)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        };
        Ok(())
    }
}
