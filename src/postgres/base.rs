use deadpool_postgres::Pool;
use std::sync::Arc;

use crate::{
    circuit_breaker::CircuitBreaker,
    error::RepositoryError,
    observer::RepositoryObserver,
};

pub struct BaseRepository {
    db: Pool,
    circuit_breaker: Arc<CircuitBreaker>,
    observer: Option<Arc<dyn RepositoryObserver>>,
}

impl BaseRepository {
    pub fn new(
        db: Pool,
        circuit_breaker: Arc<CircuitBreaker>,
        observer: Option<Arc<dyn RepositoryObserver>>,
    ) -> Self {
        Self { db, circuit_breaker, observer }
    }

    pub async fn execute_with_circuit_breaker<F, Fut, T, E>(
        &self,
        op: &str,
        table: &str,
        operation: F,
    ) -> Result<T, E>
    where
        F: FnOnce(&Pool) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let start = std::time::Instant::now();
        let result = self.circuit_breaker
            .call(|| async { operation(&self.db).await })
            .await;
        if let Some(obs) = &self.observer {
            obs.on_db_query(op, table, start.elapsed().as_secs_f64(), result.is_ok());
        }
        result
    }

    pub async fn execute_transaction<F, T, E>(&self, table: &str, f: F) -> Result<T, E>
    where
        F: for<'tx> AsyncFnOnce(&'tx tokio_postgres::Transaction<'tx>) -> Result<T, E> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let start = std::time::Instant::now();
        let result = self.circuit_breaker
            .call(|| async {
                let mut client = self
                    .db
                    .get()
                    .await
                    .map_err(|e| E::from(RepositoryError::from(e)))?;
                let tx = client
                    .transaction()
                    .await
                    .map_err(|e| E::from(RepositoryError::from(e)))?;
                match f(&tx).await {
                    Ok(v) => {
                        tx.commit()
                            .await
                            .map_err(|e| E::from(RepositoryError::from(e)))?;
                        Ok(v)
                    }
                    Err(e) => {
                        let _ = tx.rollback().await;
                        Err(e)
                    }
                }
            })
            .await;
        if let Some(obs) = &self.observer {
            obs.on_db_query("transaction", table, start.elapsed().as_secs_f64(), result.is_ok());
        }
        result
    }

    pub fn pool(&self) -> &Pool {
        &self.db
    }

    pub fn breaker_state(&self) -> crate::circuit_breaker::CircuitBreakerState {
        self.circuit_breaker.state()
    }

    #[cfg(feature = "health")]
    pub async fn check_health(&self) -> crate::health::ServiceHealth {
        use crate::health::check_database_health;
        let db = self.db.clone();
        let cb = self.circuit_breaker.clone();
        check_database_health(|| async move {
            cb.call(|| async {
                let client = db.get().await.map_err(RepositoryError::from)?;
                client
                    .query_one("SELECT 1 as health_check", &[])
                    .await
                    .map_err(RepositoryError::from)?;
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
