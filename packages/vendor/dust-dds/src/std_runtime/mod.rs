pub mod executor;

use std::time::{SystemTime, UNIX_EPOCH};

use executor::{Executor, ExecutorHandle, TimerHandle};

use crate::{
    infrastructure::time::Time,
    runtime::{Clock, DdsRuntime},
};

#[derive(Clone)]
pub struct StdClock;

impl Clock for StdClock {
    fn now(&self) -> Time {
        let now_system_time = SystemTime::now();
        let unix_time = now_system_time
            .duration_since(UNIX_EPOCH)
            .expect("Clock time is before Unix epoch start");
        Time::new(unix_time.as_secs() as i32, unix_time.subsec_nanos())
    }
}

#[derive(Default)]
pub struct StdRuntime {
    executor: Executor,
}

impl StdRuntime {
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
        }
    }
}

impl DdsRuntime for StdRuntime {
    type ClockHandle = StdClock;
    type TimerHandle = TimerHandle;
    type SpawnerHandle = ExecutorHandle;

    fn timer(&self) -> Self::TimerHandle {
        self.executor.timer_handle()
    }

    fn clock(&self) -> Self::ClockHandle {
        StdClock
    }

    fn spawner(&self) -> Self::SpawnerHandle {
        self.executor.handle()
    }
}
