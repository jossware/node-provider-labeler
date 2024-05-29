use std::time::Duration;
use time::OffsetDateTime;
use ttl_queue::TtlQueue;

#[derive(Debug)]
pub(crate) struct Diagnostics {
    pub error_count: TtlQueue<u64>,
    pub last_event: OffsetDateTime,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            // TODO: configurable duration
            error_count: TtlQueue::new(Duration::from_secs(60)),
            last_event: OffsetDateTime::now_utc(),
        }
    }
}
