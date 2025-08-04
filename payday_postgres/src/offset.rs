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
    prefix: Option<String>,
    table_name: String,
}

impl OffsetStore {
    pub fn new(db: Pool<Postgres>, table_name: Option<String>, id_prefix: Option<String>) -> Self {
        Self {
            db,
            current_offset: Mutex::new(HashMap::new()),
            prefix: id_prefix,
            table_name: table_name.unwrap_or("offsets".to_owned()),
        }
    }

    fn with_prefix(&self, id: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}:{id}"),
            None => id.to_owned(),
        }
    }

    async fn get_cached(&self, id: &str) -> Option<u64> {
        let cached = self.current_offset.lock().await;
        cached.get(&self.with_prefix(id)).copied()
    }

    async fn set_cached(&self, id: &str, offset: u64) {
        if offset <= self.get_cached(id).await.unwrap_or(0) {
            return;
        }
        let mut cached = self.current_offset.lock().await;
        cached.insert(self.with_prefix(id), offset);
    }

    fn select_query(&self) -> String {
        format!(
            "SELECT current_offset FROM {} WHERE id = $1",
            self.table_name
        )
    }

    fn upsert_query(&self) -> String {
        format!(
            "INSERT INTO {} (id, current_offset) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET current_offset = $2",
            self.table_name
        )
    }

    async fn get_offset_internal(&self, id: &str) -> Result<Option<u64>> {
        let cached = self.get_cached(id).await;
        if let Some(cached) = cached {
            return Ok(Some(cached));
        }
        let res: Option<i64> = sqlx::query(self.select_query().as_str())
            .bind(self.with_prefix(id))
            .fetch_optional(&self.db)
            .await
            .map_err(|e| Error::Db(e.to_string()))?
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
        if existing.is_none_or(|v| v <= offset) {
            sqlx::query(self.upsert_query().as_str())
                .bind(self.with_prefix(id))
                .bind(offset as i64)
                .execute(&self.db)
                .await
                .map_err(|e| Error::Db(e.to_string()))?;
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
        let store = OffsetStore::new(db, None, None);
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
        let store = OffsetStore::new(db, None, None);
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
