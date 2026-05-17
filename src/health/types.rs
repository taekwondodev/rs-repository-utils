use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub message: String,
    pub response_time_ms: Option<u64>,
}
