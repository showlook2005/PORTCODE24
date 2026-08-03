use cron_rs::chain::{delay_if_still_running, recover, skip_if_still_running, Chain};
use cron_rs::job::Job;
use cron_rs::logger::{DiscardLogger, Logger};
use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct TestLogger {
    logs: Arc<Mutex<Vec<String>>>,
}

impl Logger for TestLogger {
    fn info(&self, msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {
        self.logs.lock().unwrap().push(format!("INFO: {}", msg));
    }
    fn error(&self, err: &dyn std::error::Error, msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {
        self.logs.lock().unwrap().push(format!("ERROR: {}: {}", msg, err));
    }
}

fn appending_job(slice: Arc<Mutex<Vec<i32>>>, value: i32) -> Arc<dyn Job> {
    Arc::new(move || {
        slice.lock().unwrap().push(value);
    })
}

fn appending_wrapper(slice: Arc<Mutex<Vec<i32>>>, value: i32) -> Arc<dyn Fn(Arc<dyn Job>) -> Arc<dyn Job> + Send + Sync> {
    Arc::new(move |j: Arc<dyn Job>| {
        let slice = slice.clone();
        Arc::new(move || {
            slice.lock().unwrap().push(value);
            j.run();
        })
    })
}

#[test]
fn test_chain() {
    let nums = Arc::new(Mutex::new(Vec::new()));
    let append1 = appending_wrapper(nums.clone(), 1);
    let append2 = appending_wrapper(nums.clone(), 2);
    let append3 = appending_wrapper(nums.clone(), 3);
    let append4 = appending_job(nums.clone(), 4);

    Chain::new(vec![append1, append2, append3]).then(append4).run();
    assert_eq!(*nums.lock().unwrap(), vec![1, 2, 3, 4]);
}

#[test]
fn test_chain_recover() {
    let panicking_job: Arc<dyn Job> = Arc::new(|| {
        panic!("panickingJob panics");
    });

    // Subtest 1: panic exits job by default
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Chain::new(vec![]).then(panicking_job.clone()).run();
    }));
    assert!(res.is_err(), "panic expected, but none received");

    // Subtest 2: Recovering JobWrapper recovers
    let logs = Arc::new(Mutex::new(Vec::new()));
    let test_logger: Arc<dyn Logger> = Arc::new(TestLogger { logs: logs.clone() });
    let recovered_chain = Chain::new(vec![recover(test_logger.clone())]);
    let res2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        recovered_chain.then(panicking_job.clone()).run();
    }));
    assert!(res2.is_ok(), "expected panic to be recovered");
    assert!(!logs.lock().unwrap().is_empty(), "expected error log on panic recovery");

    // Subtest 3: composed with the *IfStillRunning wrappers
    let composed_chain = Chain::new(vec![
        recover(test_logger.clone()),
        skip_if_still_running(test_logger.clone()),
    ]);
    let res3 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        composed_chain.then(panicking_job.clone()).run();
    }));
    assert!(res3.is_ok(), "expected panic to be recovered in composed chain");
}

struct CountJob {
    started: AtomicUsize,
    done: AtomicUsize,
    delay: Duration,
}

impl CountJob {
    fn new(delay: Duration) -> Self {
        CountJob {
            started: AtomicUsize::new(0),
            done: AtomicUsize::new(0),
            delay,
        }
    }
}

impl Job for CountJob {
    fn run(&self) {
        self.started.fetch_add(1, Ordering::SeqCst);
        if self.delay > Duration::from_millis(0) {
            thread::sleep(self.delay);
        }
        self.done.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_chain_delay_if_still_running() {
    let discard: Arc<dyn Logger> = Arc::new(DiscardLogger);

    // runs immediately
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(0)));
        let wrapped = Chain::new(vec![delay_if_still_running(discard.clone())]).then(j.clone());
        let wrapped_clone = wrapped.clone();
        let handle = thread::spawn(move || wrapped_clone.run());
        handle.join().unwrap();
        assert_eq!(j.done.load(Ordering::SeqCst), 1);
    }

    // second run immediate if first done
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(0)));
        let wrapped = Chain::new(vec![delay_if_still_running(discard.clone())]).then(j.clone());
        let wrapped1 = wrapped.clone();
        let wrapped2 = wrapped.clone();
        let handle = thread::spawn(move || {
            wrapped1.run();
            thread::sleep(Duration::from_millis(1));
            wrapped2.run();
        });
        handle.join().unwrap();
        assert_eq!(j.done.load(Ordering::SeqCst), 2);
    }

    // second run delayed if first not done
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(10)));
        let wrapped = Chain::new(vec![delay_if_still_running(discard.clone())]).then(j.clone());
        let wrapped1 = wrapped.clone();
        let wrapped2 = wrapped.clone();
        let j_ref1 = j.clone();
        let j_ref2 = j.clone();

        thread::spawn(move || {
            let w1 = wrapped1.clone();
            thread::spawn(move || w1.run());
            thread::sleep(Duration::from_millis(1));
            let w2 = wrapped2.clone();
            thread::spawn(move || w2.run());
        });

        thread::sleep(Duration::from_millis(5));
        let started = j_ref1.started.load(Ordering::SeqCst);
        let done = j_ref1.done.load(Ordering::SeqCst);
        assert_eq!(started, 1);
        assert_eq!(done, 0);

        thread::sleep(Duration::from_millis(25));
        let started_final = j_ref2.started.load(Ordering::SeqCst);
        let done_final = j_ref2.done.load(Ordering::SeqCst);
        assert_eq!(started_final, 2);
        assert_eq!(done_final, 2);
    }
}

#[test]
fn test_chain_skip_if_still_running() {
    let discard: Arc<dyn Logger> = Arc::new(DiscardLogger);

    // runs immediately
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(0)));
        let wrapped = Chain::new(vec![skip_if_still_running(discard.clone())]).then(j.clone());
        wrapped.run();
        assert_eq!(j.done.load(Ordering::SeqCst), 1);
    }

    // second run immediate if first done
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(0)));
        let wrapped = Chain::new(vec![skip_if_still_running(discard.clone())]).then(j.clone());
        wrapped.run();
        thread::sleep(Duration::from_millis(1));
        wrapped.run();
        assert_eq!(j.done.load(Ordering::SeqCst), 2);
    }

    // second run skipped if first not done
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(10)));
        let wrapped = Chain::new(vec![skip_if_still_running(discard.clone())]).then(j.clone());
        let wrapped1 = wrapped.clone();
        let wrapped2 = wrapped.clone();
        let j_ref = j.clone();

        thread::spawn(move || wrapped1.run());
        thread::sleep(Duration::from_millis(1));
        thread::spawn(move || wrapped2.run());

        thread::sleep(Duration::from_millis(5));
        let started = j_ref.started.load(Ordering::SeqCst);
        let done = j_ref.done.load(Ordering::SeqCst);
        assert_eq!(started, 1);
        assert_eq!(done, 0);

        thread::sleep(Duration::from_millis(25));
        let started_final = j_ref.started.load(Ordering::SeqCst);
        let done_final = j_ref.done.load(Ordering::SeqCst);
        assert_eq!(started_final, 1);
        assert_eq!(done_final, 1);
    }

    // skip 10 jobs on rapid fire
    {
        let j = Arc::new(CountJob::new(Duration::from_millis(10)));
        let wrapped = Chain::new(vec![skip_if_still_running(discard.clone())]).then(j.clone());
        for _ in 0..11 {
            let w = wrapped.clone();
            thread::spawn(move || w.run());
        }
        thread::sleep(Duration::from_millis(200));
        assert_eq!(j.done.load(Ordering::SeqCst), 1);
    }

    // different jobs independent
    {
        let j1 = Arc::new(CountJob::new(Duration::from_millis(10)));
        let j2 = Arc::new(CountJob::new(Duration::from_millis(10)));
        let chain = Chain::new(vec![skip_if_still_running(discard.clone())]);
        let wrapped1 = chain.then(j1.clone());
        let wrapped2 = chain.then(j2.clone());

        for _ in 0..11 {
            let w1 = wrapped1.clone();
            let w2 = wrapped2.clone();
            thread::spawn(move || w1.run());
            thread::spawn(move || w2.run());
        }

        thread::sleep(Duration::from_millis(100));
        assert_eq!(j1.done.load(Ordering::SeqCst), 1);
        assert_eq!(j2.done.load(Ordering::SeqCst), 1);
    }
}
