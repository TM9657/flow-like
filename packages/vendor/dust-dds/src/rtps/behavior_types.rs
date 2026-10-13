use crate::transport::types::{Long, UnsignedLong};

// This files shall only contain the types as listed in the DDSI-RTPS Version 2.5
// Table 8.52 - Types definitions for the Behavior Module

/// Special constant value representing a zero duration
pub const DURATION_ZERO: Duration = Duration {
    seconds: 0,
    fraction: 0,
};

#[derive(Clone, Copy)]
pub struct Duration {
    seconds: Long,
    fraction: UnsignedLong,
}

impl Duration {
    pub fn new(seconds: Long, fraction: UnsignedLong) -> Self {
        Self { seconds, fraction }
    }

    pub fn seconds(&self) -> i32 {
        self.seconds
    }

    pub fn fraction(&self) -> u32 {
        self.fraction
    }

    pub fn from_millis(millis: u64) -> Self {
        let seconds = (millis / 1000) as i32;
        let fraction = ((millis % 1000) * 2u64.pow(32) / 1000) as u32;
        Self::new(seconds, fraction)
    }
}

impl From<Duration> for core::time::Duration {
    fn from(value: Duration) -> Self {
        let secs = value.seconds as u64 * 1_000_000_000;
        let nanosecs = (value.fraction as u64 * 1_000_000_000) / (1u64 << 32);
        core::time::Duration::from_nanos(secs + nanosecs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceHandle(pub [u8; 16]);
