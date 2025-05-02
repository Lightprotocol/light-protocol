use std::{fmt::Debug, num::NonZeroU32, sync::Arc, time::Duration};

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as Governor,
};
use thiserror::Error;

pub trait UseRateLimiter {
    fn set_rate_limiter(&mut self, rate_limiter: RateLimiter);
    fn rate_limiter(&self) -> Option<&RateLimiter>;
}

#[derive(Error, Debug)]
pub enum RateLimiterError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

#[derive(Clone, Debug)]
pub struct RateLimiter {
    governor: Arc<Governor<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        // Create a quota that allows exactly one request per 1/requests_per_second seconds
        let quota = Quota::with_period(Duration::from_secs_f64(1.0 / requests_per_second as f64))
            .unwrap()
            .allow_burst(NonZeroU32::new(1).unwrap());
        RateLimiter {
            governor: Arc::new(Governor::new(
                quota,
                InMemoryState::default(),
                DefaultClock::default(),
            )),
        }
    }

    pub async fn acquire(&self) -> Result<(), RateLimiterError> {
        match self.governor.check() {
            Ok(()) => Ok(()),
            Err(_) => Err(RateLimiterError::RateLimitExceeded),
        }
    }

    pub async fn acquire_with_wait(&self) {
        let _start = self.governor.until_ready().await;
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

pub struct RateLimitedClient<T> {
    inner: T,
    rate_limiter: RateLimiter,
}

impl<T> RateLimitedClient<T> {
    pub fn new(inner: T, rate_limiter: RateLimiter) -> Self {
        Self {
            inner,
            rate_limiter,
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub async fn execute<'a, F, Fut, R>(&'a self, f: F) -> Result<R, RateLimiterError>
    where
        F: FnOnce(&'a T) -> Fut + 'a,
        Fut: std::future::Future<Output = R> + 'a,
    {
        self.rate_limiter.acquire().await?;
        Ok(f(&self.inner).await)
    }

    pub async fn execute_with_wait<'a, F, Fut, R>(&'a self, f: F) -> R
    where
        F: FnOnce(&'a T) -> Fut + 'a,
        Fut: std::future::Future<Output = R> + 'a,
    {
        self.rate_limiter.acquire_with_wait().await;
        f(&self.inner).await
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(10);
        let mut successes = 0;

        for _ in 0..20 {
            if limiter.acquire().await.is_ok() {
                successes += 1;
            }
        }

        assert!(successes <= 11, "Allowed too many requests: {}", successes);
    }

    #[tokio::test]
    async fn test_rate_limited_client() {
        struct MockClient;
        impl MockClient {
            async fn make_request(&self) -> u32 {
                42
            }
        }

        let rate_limiter = RateLimiter::new(10);
        let client = RateLimitedClient::new(MockClient, rate_limiter);

        let result = client
            .execute(|c| async move { c.make_request().await })
            .await
            .unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent() {
        let rate_limiter = RateLimiter::new(10);
        let test_duration = Duration::from_secs(3);
        let start_time = Instant::now();
        let mut total_successful = 0;

        while start_time.elapsed() < test_duration {
            rate_limiter.acquire_with_wait().await;
            total_successful += 1;
        }

        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let requests_per_sec = total_successful as f64 / elapsed_secs;

        println!("Total successful requests: {}", total_successful);
        println!("Elapsed seconds: {:.2}", elapsed_secs);
        println!("Requests per second: {:.2}", requests_per_sec);

        assert!(
            requests_per_sec <= 11.0,
            "Rate should not exceed limit significantly: got {:.2} requests/sec",
            requests_per_sec
        );
        assert!(
            requests_per_sec >= 7.0,
            "Rate should be close to limit: got {:.2} requests/sec",
            requests_per_sec
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_with_wait() {
        let rate_limiter = RateLimiter::new(10);
        let start_time = Instant::now();

        for _ in 0..15 {
            rate_limiter.acquire_with_wait().await;
        }

        let elapsed = start_time.elapsed();
        println!("Elapsed time: {:?}", elapsed);

        assert!(
            elapsed >= Duration::from_millis(1400),
            "Should take close to 1.5 seconds to process all requests, took {:?}",
            elapsed
        );
    }
}
