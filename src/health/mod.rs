mod types;

pub use types::{HealthStatus, ServiceHealth};

use std::{fmt, future::Future, time::Duration};
use tokio::time::timeout;

pub async fn perform_health_check<F, Fut, E>(
    check_name: &str,
    timeout_duration: Duration,
    health_check_fn: F,
) -> ServiceHealth
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: fmt::Display,
{
    let start = std::time::Instant::now();
    let result = timeout(timeout_duration, health_check_fn()).await;
    let response_time = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(())) => ServiceHealth {
            status: HealthStatus::Healthy,
            message: format!("{check_name} connection successful"),
            response_time_ms: Some(response_time),
        },
        Ok(Err(e)) => ServiceHealth {
            status: HealthStatus::Unhealthy,
            message: format!("{check_name} error: {e}"),
            response_time_ms: Some(response_time),
        },
        Err(_) => ServiceHealth {
            status: HealthStatus::Unhealthy,
            message: format!("{check_name} connection timeout"),
            response_time_ms: None,
        },
    }
}

pub async fn check_database_health<F, Fut, E>(health_check_fn: F) -> ServiceHealth
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: fmt::Display,
{
    perform_health_check("Database", Duration::from_secs(5), health_check_fn).await
}

pub async fn check_redis_health<F, Fut, E>(health_check_fn: F) -> ServiceHealth
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: fmt::Display,
{
    perform_health_check("Redis", Duration::from_secs(5), health_check_fn).await
}
