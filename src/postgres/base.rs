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
        F: FnOnce(Pool) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let start = std::time::Instant::now();
        let result = self.circuit_breaker
            .call(|| async { operation(self.db.clone()).await })
            .await;
        if let Some(obs) = &self.observer {
            obs.on_db_query(op, table, start.elapsed().as_secs_f64(), result.is_ok());
        }
        result
    }

    pub fn pool(&self) -> &Pool {
        &self.db
    }

    /// Runs `operation` inside a transaction. Acquires a connection from the
    /// pool and passes it to `operation` **by value** (the closure owns the
    /// connection, consistent with the by-value `Pool` convention), so no
    /// borrow crosses the await boundary. Mirrors
    /// [`Self::execute_with_circuit_breaker`] — the whole transaction runs
    /// under the circuit breaker and its outcome is reported to the
    /// observer with `op`/`table`.
    ///
    /// The closure opens its own transaction with `client.transaction()`
    /// and must `commit()` on success; on error the transaction is dropped
    /// (rolled back) and the connection returns to the pool.
    ///
    /// The closure-over-`&Transaction` form would be cleaner (the helper
    /// owns commit/rollback), but it is unusable: an inline async closure's
    /// returned future is concrete, not higher-ranked over the transaction
    /// lifetime, so it cannot satisfy `for<'tx> AsyncFnOnce(&'tx Transaction)`.
    /// A named `async fn` item can, but `AsyncFnOnce` takes a single argument,
    /// so a transaction body that captures surrounding state (the normal
    /// case) cannot be expressed — see ADR-0007 in rs-server.
    pub async fn with_transaction<F, Fut, T, E>(
        &self,
        op: &str,
        table: &str,
        operation: F,
    ) -> Result<T, E>
    where
        F: FnOnce(deadpool_postgres::Object) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let start = std::time::Instant::now();
        let result = self
            .circuit_breaker
            .call(|| async {
                let client = self
                    .db
                    .get()
                    .await
                    .map_err(|e| E::from(RepositoryError::from(e)))?;
                operation(client).await
            })
            .await;
        if let Some(obs) = &self.observer {
            obs.on_db_query(op, table, start.elapsed().as_secs_f64(), result.is_ok());
        }
        result
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
