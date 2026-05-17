use redis::aio::ConnectionManager;
use std::sync::Arc;

use crate::{circuit_breaker::CircuitBreaker, error::RepositoryError};

pub struct BaseRedisRepository {
    connection_manager: ConnectionManager,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl BaseRedisRepository {
    pub fn new(connection_manager: ConnectionManager, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self { connection_manager, circuit_breaker }
    }

    pub fn breaker_state(&self) -> crate::circuit_breaker::CircuitBreakerState {
        self.circuit_breaker.state()
    }

    pub async fn execute_with_circuit_breaker<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce(ConnectionManager) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let conn = self.connection_manager.clone();
        self.circuit_breaker
            .call(|| async move { operation(conn).await })
            .await
    }

    #[cfg(feature = "health")]
    pub async fn check_health(&self) -> crate::health::ServiceHealth {
        use crate::health::check_redis_health;
        use redis::AsyncCommands;
        let conn = self.connection_manager.clone();
        let cb = self.circuit_breaker.clone();
        check_redis_health(|| async move {
            cb.call(|| async move {
                let mut conn = conn.clone();
                let _: String = conn.ping().await.map_err(RepositoryError::from)?;
                Ok::<(), RepositoryError>(())
            })
            .await
        })
        .await
    }
}
