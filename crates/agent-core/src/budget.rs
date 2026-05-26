use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Debug)]
pub struct IterationBudget {
    pub max_iterations: u32,
    pub grace_call: bool,
    iterations_used: AtomicU32,
    exhausted: AtomicBool,
}

impl IterationBudget {
    pub fn new(max_iterations: u32) -> Self {
        Self {
            max_iterations,
            grace_call: true,
            iterations_used: AtomicU32::new(0),
            exhausted: AtomicBool::new(false),
        }
    }

    pub fn exhausted(&self) -> bool {
        self.exhausted.load(Ordering::Relaxed)
    }

    pub fn increment(&self) {
        let used = self.iterations_used.fetch_add(1, Ordering::Relaxed) + 1;
        if used >= self.max_iterations {
            self.exhausted.store(true, Ordering::Relaxed);
        }
    }

    pub fn remaining(&self) -> u32 {
        let used = self.iterations_used.load(Ordering::Relaxed);
        self.max_iterations.saturating_sub(used)
    }
}
