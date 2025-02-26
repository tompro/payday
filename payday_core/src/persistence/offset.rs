use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

#[async_trait]
pub trait OffsetStoreApi: Send + Sync {
    async fn get_offset(&self, id: &str) -> Result<Offset>;
    async fn set_offset(&self, id: &str, offset: u64) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offset {
    pub id: String,
    pub offset: u64,
}
