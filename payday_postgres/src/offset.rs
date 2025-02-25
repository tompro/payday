use std::collections::HashMap;

use async_trait::async_trait;
use payday_core::{
    persistence::offset::{Offset, OffsetStoreApi},
    Error, Result,
};
use sqlx::{Pool, Postgres, Row};
use tokio::sync::Mutex;

pub struct OffsetStore {
    db: Pool<Postgres>,
    current_offset: Mutex<HashMap<String, u64>>,
}

impl OffsetStore {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self {
            db,
            current_offset: Mutex::new(HashMap::new()),
        }
    }

    async fn get_cached(&self, id: &str) -> Option<u64> {
        let cached = self.current_offset.lock().await;
        cached.get(id).copied()
    }

    async fn set_cached(&self, id: &str, offset: u64) {
        let mut cached = self.current_offset.lock().await;
        cached.insert(id.to_owned(), offset);
    }

    async fn get_offset_internal(&self, id: &str) -> Result<Option<u64>> {
        let cached = self.get_cached(id).await;
        if let Some(cached) = cached {
            return Ok(Some(cached));
        }
        let res: Option<i64> = sqlx::query("SELECT current_offset FROM offsets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?
            .map(|r| r.get("current_offset"));

        match res.and_then(|r| u64::try_from(r).ok()) {
            Some(offset) => {
                self.set_cached(id, offset).await;
                Ok(Some(offset))
            }
            _ => Ok(None),
        }
    }

    async fn set_offset_internal(&self, id: &str, offset: u64) -> Result<()> {
        let existing: Option<u64> = self.get_offset_internal(id).await?;
        if existing.is_some() {
            sqlx::query("UPDATE offsets SET current_offset = $1 WHERE id = $2")
                .bind(offset as i64)
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        } else {
            sqlx::query("INSERT INTO offsets (id, current_offset) VALUES ($1, $2)")
                .bind(id)
                .bind(offset as i64)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        }
        self.set_cached(id, offset).await;
        Ok(())
    }
}

#[async_trait]
impl OffsetStoreApi for OffsetStore {
    async fn get_offset(&self, id: &str) -> Result<Offset> {
        let offset: Option<u64> = self.get_offset_internal(id).await?;
        match offset {
            Some(offset) => Ok(Offset {
                id: id.to_owned(),
                offset,
            }),
            None => Ok(Offset {
                id: id.to_owned(),
                offset: 0,
            }),
        }
    }

    async fn set_offset(&self, id: &str, offset: u64) -> Result<()> {
        self.set_offset_internal(id, offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::get_postgres_pool;

    #[tokio::test]
    async fn test_get_set_offset_non_existant() {
        let db = get_postgres_pool().await;
        let store = OffsetStore::new(db);
        let result = store
            .get_offset("test_get_set_offset_non_existant")
            .await
            .expect("Query executed successfully");
        assert!(result.offset == 0);
    }

    #[tokio::test]
    async fn test_get_set_offset() {
        let id = "test_get_set_offset";
        let db = get_postgres_pool().await;
        let store = OffsetStore::new(db);
        store
            .set_offset(id, 10)
            .await
            .expect("Query executed successfully");

        assert!(store.current_offset.lock().await.get(id).is_some());
        assert!(store.get_cached(id).await.is_some());
        assert!(store.get_cached(id).await.unwrap().eq(&10));

        let result = store
            .get_offset(id)
            .await
            .expect("Query executed successfully");
        assert!(result.offset == 10);
    }
}
