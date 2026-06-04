use redis::aio::ConnectionManager;
use std::sync::Arc;

use crate::{
    circuit_breaker::CircuitBreaker,
    error::RepositoryError,
    observer::RepositoryObserver,
};

pub struct BaseRedisRepository {
    connection_manager: ConnectionManager,
    circuit_breaker: Arc<CircuitBreaker>,
    observer: Option<Arc<dyn RepositoryObserver>>,
}

impl BaseRedisRepository {
    pub fn new(
        connection_manager: ConnectionManager,
        circuit_breaker: Arc<CircuitBreaker>,
        observer: Option<Arc<dyn RepositoryObserver>>,
    ) -> Self {
        Self { connection_manager, circuit_breaker, observer }
    }

    pub fn breaker_state(&self) -> crate::circuit_breaker::CircuitBreakerState {
        self.circuit_breaker.state()
    }

    pub async fn execute_with_circuit_breaker<F, Fut, T, E>(
        &self,
        op: &str,
        operation: F,
    ) -> Result<T, E>
    where
        F: FnOnce(ConnectionManager) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: From<RepositoryError>,
    {
        let start = std::time::Instant::now();
        let result = self.circuit_breaker
            .call(|| async { operation(self.connection_manager.clone()).await })
            .await;
        if let Some(obs) = &self.observer {
            obs.on_redis_op(op, start.elapsed().as_secs_f64(), result.is_ok());
        }
        result
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
