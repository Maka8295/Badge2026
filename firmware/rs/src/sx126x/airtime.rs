// Rolling hourly transmit budget. The current partial minute plus the 60 before it always cover at
// least one hour, so the sum can only overstate the use in any real one-hour window.
pub const HOURLY_LIMIT_MS: u32 = 360_000;

const SECONDS_PER_BUCKET: u64 = 60;
const BUCKETS: usize = 61;

pub struct Airtime {
    used_ms: [u32; BUCKETS],
    latest_bucket: u64,
}

impl Airtime {
    pub fn new() -> Self {
        Self { used_ms: [0; BUCKETS], latest_bucket: 0 }
    }

    pub fn allows(&mut self, now_s: u64, on_air_ms: u32) -> bool {
        self.advance(now_s);
        self.used_ms.iter().sum::<u32>() + on_air_ms <= HOURLY_LIMIT_MS
    }

    pub fn record(&mut self, now_s: u64, on_air_ms: u32) {
        self.advance(now_s);
        self.used_ms[self.latest_bucket as usize % BUCKETS] += on_air_ms;
    }

    fn advance(&mut self, now_s: u64) {
        let bucket = now_s / SECONDS_PER_BUCKET;
        let first_stale = (self.latest_bucket + 1).max(bucket.saturating_sub(BUCKETS as u64 - 1));
        for stale in first_stale..=bucket {
            self.used_ms[stale as usize % BUCKETS] = 0;
        }
        self.latest_bucket = bucket;
    }
}
