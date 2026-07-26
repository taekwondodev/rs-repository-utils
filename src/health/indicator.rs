use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::health::{HealthStatus, ServiceHealth};

/// A single checkable dependency (database, cache, external API, ...).
///
/// Written by hand in dyn-safe form (`Pin<Box<dyn Future>>` rather than
/// `impl Future` in return position) since this trait is meant to be used as
/// `Arc<dyn HealthIndicator>` — the set of checkable resources in a given
/// application is open-ended and heterogeneous, unlike most repository/service
/// ports which should stay generic bounds for zero-cost static dispatch.
pub trait HealthIndicator: Send + Sync {
    /// Stable key used in the aggregated report (e.g. "database", "redis").
    fn name(&self) -> &'static str;

    fn check(&self) -> Pin<Box<dyn Future<Output = ServiceHealth> + Send + '_>>;
}

#[derive(Debug)]
pub struct HealthReport {
    pub checks: BTreeMap<Box<str>, ServiceHealth>,
}

impl HealthReport {
    pub fn is_healthy(&self) -> bool {
        self.checks.values().all(|h| h.status == HealthStatus::Healthy)
    }
}

/// Runs every indicator concurrently and assembles the report. Takes a
/// dynamic, heterogeneous slice on purpose: the set of checkable resources
/// varies per application and can't be pinned to a compile-time type without
/// every caller enumerating every adapter.
///
/// No timestamp on `HealthReport` by design — "when was this report served"
/// is a presentation concern for whichever layer turns this into an HTTP
/// response, not something this library should need a time-formatting
/// dependency for.
pub async fn check_all(indicators: &[Arc<dyn HealthIndicator>]) -> HealthReport {
    let mut set = tokio::task::JoinSet::new();
    for indicator in indicators {
        let indicator = Arc::clone(indicator);
        set.spawn(async move { (indicator.name(), indicator.check().await) });
    }

    let mut checks = BTreeMap::new();
    while let Some(result) = set.join_next().await {
        if let Ok((name, health)) = result {
            checks.insert(Box::<str>::from(name), health);
        }
    }

    HealthReport { checks }
}
