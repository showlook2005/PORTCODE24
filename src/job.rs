use std::sync::Arc;

/// Job is an interface for submitted cron jobs.
pub trait Job: Send + Sync {
    fn run(&self);
}

/// Blanket implementation of Job for any function closure.
impl<F> Job for F
where
    F: Fn() + Send + Sync,
{
    fn run(&self) {
        (self)();
    }
}

/// Arc<dyn Job> can also be run.
impl Job for Arc<dyn Job> {
    fn run(&self) {
        (**self).run();
    }
}
