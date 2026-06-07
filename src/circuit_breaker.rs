use failsafe::{CircuitBreaker as FailsafeCircuitBreaker, Config, StateMachine, backoff, failure_policy};
use std::{sync::Arc, time::Duration};

use crate::error::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BreakerState {
    Closed,
    Open,
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub backoff_initial_secs: u64,
    pub backoff_max_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            backoff_initial_secs: 10,
            backoff_max_secs: 60,
        }
    }
}

type BreakerImpl = StateMachine<
    failsafe::failure_policy::ConsecutiveFailures<failsafe::backoff::EqualJittered>,
    (),
>;

#[derive(Clone)]
pub struct CircuitBreaker {
    breaker: Arc<BreakerImpl>,
    name: Box<str>,
}

impl CircuitBreaker {
    pub fn new(name: &str, config: CircuitBreakerConfig) -> Self {
        let backoff_strategy = backoff::equal_jittered(
            Duration::from_secs(config.backoff_initial_secs),
            Duration::from_secs(config.backoff_max_secs),
        );
        let policy = failure_policy::consecutive_failures(config.failure_threshold, backoff_strategy);
        let breaker = Config::new().failure_policy(policy).build();

        let cb = Self { breaker: Arc::new(breaker), name: name.into() };
        cb.log_state(BreakerState::Closed);
        cb
    }

    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: From<RepositoryError>,
    {
        if self.is_open() {
            self.log_state(BreakerState::Open);
            return Err(E::from(RepositoryError::CircuitBreakerOpen(
                format!("service '{}' is temporarily unavailable", self.name).into(),
            )));
        }

        match f().await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(error) => {
                self.record_failure();
                Err(error)
            }
        }
    }

    fn log_state(&self, state: BreakerState) {
        match state {
            BreakerState::Closed => {
                tracing::debug!(circuit_breaker = %self.name, "State: CLOSED");
            }
            BreakerState::Open => {
                tracing::error!(circuit_breaker = %self.name, "State: OPEN — rejecting requests");
            }
        }
    }

    pub fn state(&self) -> CircuitBreakerState {
        if self.is_open() {
            CircuitBreakerState::Open
        } else {
            CircuitBreakerState::Closed
        }
    }

    fn is_open(&self) -> bool {
        self.breaker.call(|| Ok::<_, ()>(())).is_err()
    }

    fn record_success(&self) {
        let _ = self.breaker.call(|| Ok::<_, ()>(()));
        self.log_state(BreakerState::Closed);
    }

    fn record_failure(&self) {
        let check_result = self.breaker.call(|| Err::<(), _>(()));
        let just_opened = matches!(check_result, Err(failsafe::Error::Rejected));

        if just_opened {
            self.log_state(BreakerState::Open);
        } else {
            tracing::warn!(circuit_breaker = %self.name, "Failure recorded");
        }
    }
}
