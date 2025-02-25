use async_trait::async_trait;
use payday_core::{
    persistence::offset::{Offset, OffsetStoreApi},
    Error, Result,
};
use sqlx::{Pool, Postgres, Row};
use tokio::sync::Mutex;

pub struct OffsetStore {
    db: Pool<Postgres>,
    id: String,
    current_offset: Box<Mutex<Option<u64>>>,
}

impl OffsetStore {
    pub fn new(db: Pool<Postgres>, id: &str) -> Self {
        Self {
            db,
            id: id.to_string(),
            current_offset: Box::new(Mutex::new(None)),
        }
    }

    async fn get_cached(&self) -> Option<u64> {
        let cached = self.current_offset.lock().await;
        *cached
    }

    async fn set_cached(&self, offset: u64) {
        let mut cached = self.current_offset.lock().await;
        *cached = Some(offset);
    }

    async fn get_offset_internal(&self) -> Result<Option<u64>> {
        let cached = self.get_cached().await;
        if let Some(cached) = cached {
            return Ok(Some(cached));
        }
        let res: Option<i64> = sqlx::query("SELECT current_offset FROM offsets WHERE id = $1")
            .bind(&self.id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?
            .map(|r| r.get("current_offset"));

        match res.and_then(|r| u64::try_from(r).ok()) {
            Some(offset) => {
                self.set_cached(offset).await;
                Ok(Some(offset))
            }
            _ => Ok(None),
        }
    }

    async fn set_offset_internal(&self, offset: u64) -> Result<()> {
        let existing: Option<u64> = self.get_offset_internal().await?;
        if existing.is_some() {
            sqlx::query("UPDATE offsets SET current_offset = $1 WHERE id = $2")
                .bind(offset as i64)
                .bind(&self.id)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        } else {
            sqlx::query("INSERT INTO offsets (id, current_offset) VALUES ($1, $2)")
                .bind(&self.id)
                .bind(offset as i64)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
        }
        self.set_cached(offset).await;
        Ok(())
    }
}

#[async_trait]
impl OffsetStoreApi for OffsetStore {
    async fn get_offset(&self) -> Result<Offset> {
        let offset: Option<u64> = self.get_offset_internal().await?;
        match offset {
            Some(offset) => Ok(Offset {
                id: self.id.to_owned(),
                offset,
            }),
            None => Ok(Offset {
                id: self.id.to_owned(),
                offset: 0,
            }),
        }
    }

    async fn set_offset(&self, offset: u64) -> Result<()> {
        self.set_offset_internal(offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::get_postgres_pool;

    #[tokio::test]
    async fn test_get_set_offset_non_existant() {
        let db = get_postgres_pool().await;
        let store = OffsetStore::new(db, "test_get_set_offset_non_existant");
        let result = store
            .get_offset()
            .await
            .expect("Query executed successfully");
        assert!(result.offset == 0);
    }

    #[tokio::test]
    async fn test_get_set_offset() {
        let db = get_postgres_pool().await;
        let store = OffsetStore::new(db, "test_get_set_offset");
        store
            .set_offset(10)
            .await
            .expect("Query executed successfully");

        assert!(store.current_offset.lock().await.is_some());
        assert!(store.get_cached().await.is_some());
        assert!(store.get_cached().await.unwrap().eq(&10));

        let result = store
            .get_offset()
            .await
            .expect("Query executed successfully");
        assert!(result.offset == 10);
    }
}
