use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::PaydayResult;

#[async_trait]
pub trait BlockHeightStoreApi: Send + Sync {
    async fn get_block_height(&self, node_id: &str) -> PaydayResult<BlockHeight>;
    async fn set_block_height(&self, node_id: &str, block_height: u64) -> PaydayResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeight {
    pub node_id: String,
    pub block_height: u64,
}
