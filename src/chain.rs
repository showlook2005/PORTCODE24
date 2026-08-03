use crate::job::Job;
use crate::logger::Logger;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type JobWrapper = Arc<dyn Fn(Arc<dyn Job>) -> Arc<dyn Job> + Send + Sync>;

#[derive(Clone)]
pub struct Chain {
    wrappers: Vec<JobWrapper>,
}

impl Chain {
    pub fn new(wrappers: Vec<JobWrapper>) -> Self {
        Chain { wrappers }
    }

    pub fn then(&self, job: Arc<dyn Job>) -> Arc<dyn Job> {
        let mut j = job;
        for w in self.wrappers.iter().rev() {
            j = w(j.clone());
        }
        j
    }
}

pub fn recover(logger: Arc<dyn Logger>) -> JobWrapper {
    Arc::new(move |j: Arc<dyn Job>| {
        let logger = logger.clone();
        Arc::new(move || {
            let j = j.clone();
            let logger = logger.clone();
            let result = catch_unwind(AssertUnwindSafe(move || {
                j.run();
            }));
            if let Err(err_payload) = result {
                let msg = if let Some(s) = err_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = err_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                let err = std::io::Error::new(std::io::ErrorKind::Other, msg);
                logger.error(&err, "panic", &[]);
            }
        })
    })
}

pub fn delay_if_still_running(logger: Arc<dyn Logger>) -> JobWrapper {
    Arc::new(move |j: Arc<dyn Job>| {
        let logger = logger.clone();
        let mu = Arc::new(Mutex::new(()));
        Arc::new(move || {
            let start = Instant::now();
            let _guard = mu.lock().unwrap();
            let dur = start.elapsed();
            if dur > Duration::from_secs(60) {
                logger.info("delay", &[("duration", &dur.as_secs_f64())]);
            }
            j.run();
        })
    })
}

pub fn skip_if_still_running(logger: Arc<dyn Logger>) -> JobWrapper {
    Arc::new(move |j: Arc<dyn Job>| {
        let logger = logger.clone();
        let running = Arc::new(AtomicBool::new(false));
        Arc::new(move || {
            if running
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                struct Guard(Arc<AtomicBool>);
                impl Drop for Guard {
                    fn drop(&mut self) {
                        self.0.store(false, Ordering::SeqCst);
                    }
                }
                let _guard = Guard(running.clone());
                j.run();
            } else {
                logger.info("skip", &[]);
            }
        })
    })
}
