use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Duration;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strategy(&self) -> RetryStrategy {
        RetryStrategy {
            policy: self.clone(),
            attempts: 0,
            rng: self.jitter.then(StdRng::from_entropy),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryStrategy {
    policy: RetryPolicy,
    attempts: usize,
    rng: Option<StdRng>,
}

impl RetryStrategy {
    pub fn attempts(&self) -> usize {
        self.attempts
    }

    pub fn next_delay(&mut self, error: &Error) -> Option<Duration> {
        if self.attempts >= self.policy.max_attempts || !error.is_retryable() {
            return None;
        }

        self.attempts += 1;

        if let Some(retry_after) = error.retry_after() {
            return Some(retry_after.min(self.policy.max_delay));
        }

        let exp = self
            .policy
            .exponential_base
            .powi((self.attempts - 1) as i32);
        let mut delay = self.policy.initial_delay.mul_f64(exp);
        if delay > self.policy.max_delay {
            delay = self.policy.max_delay;
        }

        if let Some(rng) = &mut self.rng {
            let jitter: f64 = rng.gen_range(0.5..1.5);
            delay = delay.mul_f64(jitter).min(self.policy.max_delay);
        }

        Some(delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use anyhow::anyhow;

    fn base_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
            jitter: false,
        }
    }

    #[test]
    fn retryable_error_uses_exponential_backoff() {
        let policy = base_policy();
        let mut strategy = policy.strategy();

        let delay1 = strategy.next_delay(&Error::Timeout).expect("first retry");
        let delay2 = strategy.next_delay(&Error::Timeout).expect("second retry");
        let delay3 = strategy.next_delay(&Error::Timeout).expect("third retry");
        let delay4 = strategy.next_delay(&Error::Timeout);

        assert_eq!(delay1, Duration::from_millis(200));
        assert_eq!(delay2, Duration::from_millis(400));
        assert_eq!(delay3, Duration::from_millis(800));
        assert!(delay4.is_none());
        assert_eq!(strategy.attempts(), 3);
    }

    #[test]
    fn non_retryable_error_returns_none() {
        let policy = base_policy();
        let mut strategy = policy.strategy();

        assert!(strategy
            .next_delay(&Error::InvalidRequest("bad".into()))
            .is_none());
        assert_eq!(strategy.attempts(), 0);
    }

    #[test]
    fn retry_after_value_is_respected() {
        let mut policy = base_policy();
        policy.max_delay = Duration::from_secs(1);
        let mut strategy = policy.strategy();

        let err = Error::Provider {
            provider: "stub".into(),
            source: anyhow!("upstream failure"),
            retry_after: Some(Duration::from_secs(5)),
            http: None,
        };

        let delay = strategy.next_delay(&err).expect("retry_after delay");
        assert_eq!(delay, Duration::from_secs(1));
    }

    #[test]
    fn jitter_stays_within_expected_bounds() {
        let mut policy = base_policy();
        policy.jitter = true;
        let mut strategy = policy.strategy();

        let delay = strategy.next_delay(&Error::Timeout).expect("jitter delay");
        assert!(delay >= Duration::from_millis(100));
        assert!(delay <= Duration::from_millis(300));
    }
}
