pub mod circuit_breaker;
pub mod error;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "health")]
pub mod health;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use error::RepositoryError;

#[cfg(feature = "postgres")]
pub use postgres::{BaseRepository, DeleteBuilder, FromRow, InsertBuilder, OrderDirection, RepositoryMetrics, SelectBuilder, UpdateBuilder};

#[cfg(feature = "redis")]
pub use redis::BaseRedisRepository;

#[cfg(feature = "health")]
pub use health::{HealthStatus, ServiceHealth, check_database_health, check_redis_health, perform_health_check};
