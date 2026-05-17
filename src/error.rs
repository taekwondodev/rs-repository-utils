use std::fmt;

#[derive(Debug)]
pub enum RepositoryError {
    Pool(String),
    Query(String),
    Redis(String),
    Cache(String),
    CircuitBreakerOpen(String),
    InvalidQuery(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(msg) => write!(f, "pool error: {msg}"),
            Self::Query(msg) => write!(f, "query error: {msg}"),
            Self::Redis(msg) => write!(f, "redis error: {msg}"),
            Self::Cache(msg) => write!(f, "cache error: {msg}"),
            Self::CircuitBreakerOpen(msg) => write!(f, "circuit breaker open: {msg}"),
            Self::InvalidQuery(msg) => write!(f, "invalid query: {msg}"),
        }
    }
}

impl std::error::Error for RepositoryError {}

#[cfg(feature = "postgres")]
impl From<deadpool_postgres::PoolError> for RepositoryError {
    fn from(e: deadpool_postgres::PoolError) -> Self {
        Self::Pool(e.to_string())
    }
}

#[cfg(feature = "postgres")]
impl From<tokio_postgres::Error> for RepositoryError {
    fn from(e: tokio_postgres::Error) -> Self {
        Self::Query(e.to_string())
    }
}

#[cfg(feature = "redis")]
impl From<redis::RedisError> for RepositoryError {
    fn from(e: redis::RedisError) -> Self {
        Self::Redis(e.to_string())
    }
}
