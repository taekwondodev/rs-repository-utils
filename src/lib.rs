pub mod circuit_breaker;
pub mod error;
pub mod observer;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "health")]
pub mod health;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
pub use error::RepositoryError;
pub use observer::RepositoryObserver;

#[cfg(feature = "postgres")]
pub use postgres::{BaseRepository, DeleteBuilder, FromRow, InsertBuilder, OrderDirection, SelectBuilder, UpdateBuilder};

#[cfg(feature = "redis")]
pub use redis::BaseRedisRepository;

#[cfg(feature = "health")]
pub use health::{
    HealthIndicator, HealthReport, HealthStatus, ServiceHealth, check_all, check_database_health,
    check_redis_health, perform_health_check,
};
