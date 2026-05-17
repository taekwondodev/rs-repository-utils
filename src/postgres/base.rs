use deadpool_postgres::Pool;
use std::sync::Arc;
use tokio_postgres::types::ToSql;

use crate::{circuit_breaker::CircuitBreaker, error::RepositoryError};
use super::prepared_cache::PreparedStatementCache;

pub struct BaseRepository {
    db: Pool,
    circuit_breaker: Arc<CircuitBreaker>,
    prepared_cache: PreparedStatementCache,
}

impl BaseRepository {
    pub fn new(db: Pool, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            db,
            circuit_breaker,
            prepared_cache: PreparedStatementCache::new(),
        }
    }

    pub async fn execute_with_circuit_breaker<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce(Pool) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let db = self.db.clone();
        self.circuit_breaker
            .call(|| async move { operation(db).await })
            .await
    }

    pub async fn execute_prepared(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, RepositoryError> {
        let client = self.db.get().await?;
        let stmt = self.prepared_cache.get_or_prepare(&client, query).await?;
        Ok(client.query(&stmt, params).await?)
    }

    pub async fn execute_prepared_one(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<tokio_postgres::Row, RepositoryError> {
        let client = self.db.get().await?;
        let stmt = self.prepared_cache.get_or_prepare(&client, query).await?;
        Ok(client.query_one(&stmt, params).await?)
    }

    pub async fn execute_prepared_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<tokio_postgres::Row>, RepositoryError> {
        let client = self.db.get().await?;
        let stmt = self.prepared_cache.get_or_prepare(&client, query).await?;
        Ok(client.query_opt(&stmt, params).await?)
    }

    pub async fn execute_prepared_raw(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, RepositoryError> {
        let client = self.db.get().await?;
        let stmt = self.prepared_cache.get_or_prepare(&client, query).await?;
        Ok(client.execute(&stmt, params).await?)
    }

    pub fn pool(&self) -> &Pool {
        &self.db
    }

    #[cfg(feature = "health")]
    pub async fn check_health(&self) -> crate::health::ServiceHealth {
        use crate::health::check_database_health;
        let db = self.db.clone();
        let cb = self.circuit_breaker.clone();
        check_database_health(|| async move {
            cb.call(|| async {
                let client = db.get().await.map_err(RepositoryError::from)?;
                client.query_one("SELECT 1 as health_check", &[]).await.map_err(RepositoryError::from)?;
                Ok::<(), RepositoryError>(())
            })
            .await
        })
        .await
    }
}

pub trait FromRow: Sized {
    fn from_row(row: &tokio_postgres::Row) -> Result<Self, RepositoryError>;
}

/// Implement to expose pool metrics to your observability layer.
pub trait RepositoryMetrics {
    fn update_pool_metrics(&self);
}
