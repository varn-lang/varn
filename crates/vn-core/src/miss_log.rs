use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Default)]
pub struct MissLog(AtomicU32);

impl MissLog {
    pub fn record(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn count(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Clone for MissLog {
    fn clone(&self) -> Self {
        Self(AtomicU32::new(self.count()))
    }
}
