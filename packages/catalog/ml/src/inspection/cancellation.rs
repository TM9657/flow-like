use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Runs `action` when dropped unless disarmed. The executor drops a node's future when its run
/// is cancelled, so a guard held across the await is what reaches detached blocking work.
#[must_use = "dropping the guard runs its action immediately"]
pub struct CancelOnDrop<F: FnOnce()> {
    action: Option<F>,
}

impl<F: FnOnce()> CancelOnDrop<F> {
    pub fn new(action: F) -> Self {
        Self {
            action: Some(action),
        }
    }

    pub fn disarm(mut self) {
        self.action = None;
    }
}

impl<F: FnOnce()> Drop for CancelOnDrop<F> {
    fn drop(&mut self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }
}

#[derive(Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn dropping_an_armed_guard_runs_its_action_once() {
        let runs = Cell::new(0);
        let guard = CancelOnDrop::new(|| runs.set(runs.get() + 1));
        assert_eq!(runs.get(), 0);
        drop(guard);
        assert_eq!(runs.get(), 1);
    }

    #[test]
    fn a_disarmed_guard_never_runs_its_action() {
        let runs = Cell::new(0);
        CancelOnDrop::new(|| runs.set(runs.get() + 1)).disarm();
        assert_eq!(runs.get(), 0);
    }

    #[test]
    fn a_guard_cancels_a_flag_shared_across_clones() {
        let flag = CancelFlag::default();
        let worker = flag.clone();
        assert!(!worker.is_cancelled());
        let action = flag.clone();
        drop(CancelOnDrop::new(move || action.cancel()));
        assert!(worker.is_cancelled());
        assert!(flag.is_cancelled());
        assert!(flag.clone().is_cancelled());
    }
}
