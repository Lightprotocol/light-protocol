use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

pub struct BackpressureControl {
    semaphore: Arc<Semaphore>,
}

impl BackpressureControl {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(limit)),
        }
    }

    pub(crate) async fn acquire(&self) -> SemaphorePermit {
        self.semaphore.acquire().await.unwrap()
    }
}
