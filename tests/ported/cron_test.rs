use cron_rs::chain::recover;
use cron_rs::cron::*;
use cron_rs::job::Job;
use cron_rs::logger::Logger;
use cron_rs::parser::{parse_option, Parser};
use cron_rs::schedule::Schedule;
use chrono::{DateTime, Datelike, Timelike};
use chrono_tz::Tz;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ONE_SECOND: Duration = Duration::from_millis(1050);

struct SyncLogger {
    logs: Arc<Mutex<Vec<String>>>,
}

impl Logger for SyncLogger {
    fn info(&self, msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {
        self.logs.lock().unwrap().push(format!("INFO: {}", msg));
    }
    fn error(&self, err: &dyn std::error::Error, msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {
        self.logs.lock().unwrap().push(format!("ERROR: {}: {}", msg, err));
    }
}

fn second_parser() -> Parser {
    Parser::new(
        parse_option::SECOND
            | parse_option::MINUTE
            | parse_option::HOUR
            | parse_option::DOM
            | parse_option::MONTH
            | parse_option::DOW_OPTIONAL
            | parse_option::DESCRIPTOR,
    )
}

#[tokio::test]
async fn test_func_panic_recovery() {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logger: Arc<dyn Logger> = Arc::new(SyncLogger { logs: logs.clone() });

    let cron = Cron::with_options(vec![
        OptionSetter::Parser(second_parser()),
        OptionSetter::Chain(cron_rs::chain::Chain::new(vec![recover(logger)])),
    ]);

    cron.start();
    let _ = cron.add_func("* * * * * ?", || {
        panic!("YOLO");
    });

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    let output = logs.lock().unwrap().join("\n");
    assert!(output.contains("YOLO"), "expected panic to be logged, got: {}", output);
}

struct DummyJob;
impl Job for DummyJob {
    fn run(&self) {
        panic!("YOLO");
    }
}

#[tokio::test]
async fn test_job_panic_recovery() {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logger: Arc<dyn Logger> = Arc::new(SyncLogger { logs: logs.clone() });

    let cron = Cron::with_options(vec![
        OptionSetter::Parser(second_parser()),
        OptionSetter::Chain(cron_rs::chain::Chain::new(vec![recover(logger)])),
    ]);

    cron.start();
    let _ = cron.add_job("* * * * * ?", Arc::new(DummyJob));

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    let output = logs.lock().unwrap().join("\n");
    assert!(output.contains("YOLO"), "expected panic to be logged, got: {}", output);
}

#[tokio::test]
async fn test_no_entries() {
    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    cron.start();
    let rx = cron.stop();
    let res = tokio::time::timeout(ONE_SECOND, rx).await;
    assert!(res.is_ok(), "expected cron will be stopped immediately");
}

#[tokio::test]
async fn test_stop_causes_jobs_to_not_run() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    cron.start();
    cron.stop();

    let _ = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    });

    tokio::time::sleep(ONE_SECOND).await;
    assert!(!ran.load(Ordering::SeqCst), "expected stopped cron does not run any job");
}

#[tokio::test]
async fn test_add_before_running() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    });

    cron.start();
    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert!(ran.load(Ordering::SeqCst), "expected job runs");
}

#[tokio::test]
async fn test_add_while_running() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    cron.start();

    let _ = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    });

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert!(ran.load(Ordering::SeqCst), "expected job runs");
}

#[tokio::test]
async fn test_remove_before_running() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let id = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    }).unwrap();

    cron.remove(id);
    cron.start();

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert!(!ran.load(Ordering::SeqCst), "expected removed job does not run");
}

#[tokio::test]
async fn test_remove_while_running() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    cron.start();

    let id = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    }).unwrap();

    cron.remove(id);

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert!(!ran.load(Ordering::SeqCst), "expected removed job does not run");
}

#[tokio::test]
async fn test_snapshot_entries() {
    let cron = Cron::new();
    let _ = cron.add_func("@every 2s", || {});
    cron.start();

    tokio::time::sleep(Duration::from_secs(1)).await;
    let entries = cron.entries();
    assert_eq!(entries.len(), 1);

    cron.stop();
}

#[tokio::test]
async fn test_invalid_job_spec() {
    let cron = Cron::new();
    let res = cron.add_job("this will not parse", Arc::new(DummyJob));
    assert!(res.is_err(), "expected an error with invalid spec");
}

#[tokio::test]
async fn test_stop_without_start() {
    let cron = Cron::new();
    let _ = cron.stop();
}

#[derive(Debug)]
struct ZeroSchedule;
impl Schedule for ZeroSchedule {
    fn next(&self, _after: DateTime<Tz>) -> Option<DateTime<Tz>> {
        None
    }
}

#[tokio::test]
async fn test_job_with_zero_time_does_not_run() {
    let calls = Arc::new(AtomicI64::new(0));
    let calls_clone = calls.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func("* * * * * *", move || {
        calls_clone.fetch_add(1, Ordering::SeqCst);
    });
    cron.schedule(Arc::new(ZeroSchedule), Arc::new(|| {
        panic!("expected zero task will not run");
    }));

    cron.start();
    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_stop_and_wait() {
    // subtest: nothing running, returns immediately
    {
        let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
        cron.start();
        let rx = cron.stop();
        let res = tokio::time::timeout(Duration::from_millis(50), rx).await;
        assert!(res.is_ok(), "context was not done immediately");
    }

    // subtest: repeated calls to stop
    {
        let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
        cron.start();
        let _ = cron.stop();
        tokio::time::sleep(Duration::from_millis(1)).await;
        let rx = cron.stop();
        let res = tokio::time::timeout(Duration::from_millis(50), rx).await;
        assert!(res.is_ok(), "context was not done immediately on repeated stop");
    }
}

// NOTE: TestWithLocation (option_test.go) lives in tests/ported/option_test.rs,
// alongside TestWithParser and TestWithVerboseLogger, matching the Go file layout.

// Port of TestAddWhileRunningWithDelay (regression test for #34: adding a job
// after Start() should not cause multiple invocations).
#[tokio::test]
async fn test_add_while_running_with_delay() {
    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    cron.start();
    tokio::time::sleep(Duration::from_secs(5)).await;

    let calls = Arc::new(AtomicI64::new(0));
    let calls_clone = calls.clone();
    let _ = cron.add_func("* * * * * *", move || {
        calls_clone.fetch_add(1, Ordering::SeqCst);
    });

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert_eq!(calls.load(Ordering::SeqCst), 1, "called {} times, expected 1", calls.load(Ordering::SeqCst));
}

// Port of TestMultipleEntries: entries are correctly sorted, an immediate
// entry runs immediately, and multiple jobs can run in the same instant.
#[tokio::test]
async fn test_multiple_entries() {
    let ran = Arc::new(Mutex::new(0u32));

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func("0 0 0 1 1 ?", || {});
    {
        let ran = ran.clone();
        let _ = cron.add_func("* * * * * ?", move || {
            *ran.lock().unwrap() += 1;
        });
    }
    let id1 = cron.add_func("* * * * * ?", || panic!("should have been removed")).unwrap();
    let id2 = cron.add_func("* * * * * ?", || panic!("should have been removed")).unwrap();
    let _ = cron.add_func("0 0 0 31 12 ?", || {});
    {
        let ran = ran.clone();
        let _ = cron.add_func("* * * * * ?", move || {
            *ran.lock().unwrap() += 1;
        });
    }

    cron.remove(id1);
    cron.start();
    cron.remove(id2);

    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    assert_eq!(*ran.lock().unwrap(), 2, "expected job run in proper order");
}

// Port of TestRunningJobTwice: a per-second job should fire (at least) twice
// within two ticks while far-future entries never fire.
#[tokio::test]
async fn test_running_job_twice() {
    let ran = Arc::new(AtomicI64::new(0));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func("0 0 0 1 1 ?", || {});
    let _ = cron.add_func("0 0 0 31 12 ?", || {});
    let _ = cron.add_func("* * * * * ?", move || {
        ran_clone.fetch_add(1, Ordering::SeqCst);
    });

    cron.start();
    tokio::time::sleep(ONE_SECOND * 2).await;
    cron.stop();

    assert!(ran.load(Ordering::SeqCst) >= 2, "expected job fires 2 times");
}

// Port of TestRunningMultipleSchedules: mixes cron-spec entries with
// Schedule-based entries (Every) and confirms the fast one fires.
#[tokio::test]
async fn test_running_multiple_schedules() {
    let ran = Arc::new(AtomicI64::new(0));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func("0 0 0 1 1 ?", || {});
    let _ = cron.add_func("0 0 0 31 12 ?", || {});
    let _ = cron.add_func("* * * * * ?", {
        let ran = ran.clone();
        move || {
            ran.fetch_add(1, Ordering::SeqCst);
        }
    });
    cron.schedule(Arc::new(cron_rs::constant_delay::every(chrono::Duration::minutes(1))), Arc::new(|| {}));
    cron.schedule(
        Arc::new(cron_rs::constant_delay::every(chrono::Duration::seconds(1))),
        Arc::new(move || {
            ran_clone.fetch_add(1, Ordering::SeqCst);
        }),
    );
    cron.schedule(Arc::new(cron_rs::constant_delay::every(chrono::Duration::hours(1))), Arc::new(|| {}));

    cron.start();
    tokio::time::sleep(ONE_SECOND * 2).await;
    cron.stop();

    assert!(ran.load(Ordering::SeqCst) >= 2, "expected job fires 2 times");
}

// Port of TestLocalTimezone: cron runs in the local timezone (as opposed to UTC).
#[tokio::test]
async fn test_local_timezone() {
    let ran = Arc::new(AtomicI64::new(0));
    let ran_clone = ran.clone();

    let mut now = chrono::Local::now();
    // Matches the Go fix for issue #205: this calculation doesn't work in
    // seconds 58/59, so just sleep past them.
    if now.second() >= 58 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        now = chrono::Local::now();
    }
    let spec = format!(
        "{},{} {} {} {} {} ?",
        now.second() + 1,
        now.second() + 2,
        now.minute(),
        now.hour(),
        now.day(),
        now.month()
    );

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func(&spec, move || {
        ran_clone.fetch_add(1, Ordering::SeqCst);
    });
    cron.start();
    tokio::time::sleep(ONE_SECOND * 2).await;
    cron.stop();

    assert!(ran.load(Ordering::SeqCst) >= 2, "expected job fires 2 times");
}

// Port of TestNonLocalTimezone: cron runs in an explicitly configured timezone.
#[tokio::test]
async fn test_non_local_timezone() {
    let ran = Arc::new(AtomicI64::new(0));
    let ran_clone = ran.clone();

    let loc = chrono_tz::Tz::Atlantic__Cape_Verde;
    let mut now = chrono::Utc::now().with_timezone(&loc);
    if now.second() >= 58 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        now = chrono::Utc::now().with_timezone(&loc);
    }
    let spec = format!(
        "{},{} {} {} {} {} ?",
        now.second() + 1,
        now.second() + 2,
        now.minute(),
        now.hour(),
        now.day(),
        now.month()
    );

    let cron = Cron::with_options(vec![
        OptionSetter::Location(loc),
        OptionSetter::Parser(second_parser()),
    ]);
    let _ = cron.add_func(&spec, move || {
        ran_clone.fetch_add(1, Ordering::SeqCst);
    });
    cron.start();
    tokio::time::sleep(ONE_SECOND * 2).await;
    cron.stop();

    assert!(ran.load(Ordering::SeqCst) >= 2, "expected job fires 2 times");
}

// Port of TestStopWithoutStart is already covered by test_stop_without_start above.

// Port of TestBlockingRun: the blocking `run()` method behaves like `start()`
// except it does not return until stopped.
#[tokio::test]
async fn test_blocking_run() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Arc::new(Cron::with_options(vec![OptionSetter::Parser(second_parser())]));
    let _ = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    });

    let unblocked = Arc::new(AtomicBool::new(false));
    let unblocked_clone = unblocked.clone();
    let cron_clone = cron.clone();
    let handle = tokio::task::spawn_blocking(move || {
        cron_clone.run();
        unblocked_clone.store(true, Ordering::SeqCst);
    });

    tokio::time::sleep(ONE_SECOND).await;
    assert!(ran.load(Ordering::SeqCst), "expected job fires");
    assert!(!unblocked.load(Ordering::SeqCst), "expected that run() blocks");

    cron.stop();
    let _ = handle.await;
}

// Port of TestStartNoop: calling start() twice is a no-op; job fires exactly
// once per tick, not twice.
#[tokio::test]
async fn test_start_noop() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(4);

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let _ = cron.add_func("* * * * * ?", move || {
        let _ = tx.try_send(());
    });

    cron.start();
    // Wait for the first firing to ensure the runner is going.
    rx.recv().await;

    cron.start(); // no-op, cron already running

    rx.recv().await;

    // Fail if a third tick arrives immediately, indicating a double-run.
    let extra = tokio::time::timeout(Duration::from_millis(1), rx.recv()).await;
    cron.stop();
    assert!(extra.is_err(), "expected job fires exactly twice per tick, not more");
}

// Port of TestJob: entries are returned via `entry()`/`entries()` and sorted
// by upcoming run time. Rust trait objects can't be downcast to recover the
// original job's identity the way Go's type assertion does, so this port
// tracks ordering via EntryID instead of a job `name` field.
#[tokio::test]
async fn test_job() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran_clone = ran.clone();

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let id0 = cron.add_func("0 0 0 30 2 ?", || {}).unwrap(); // Feb 30 never matches
    let id1 = cron.add_func("0 0 0 1 1 ?", || {}).unwrap();
    let id2 = cron.add_func("* * * * * ?", move || {
        ran_clone.store(true, Ordering::SeqCst);
    }).unwrap();
    let id3 = cron.add_func("1 0 0 1 1 ?", || {}).unwrap();
    let id4 = cron.schedule(
        Arc::new(cron_rs::constant_delay::every(chrono::Duration::seconds(5))),
        Arc::new(|| {}),
    );
    let id5 = cron.schedule(
        Arc::new(cron_rs::constant_delay::every(chrono::Duration::minutes(5))),
        Arc::new(|| {}),
    );

    // Test getting an Entry pre-Start.
    assert_eq!(cron.entry(id2).map(|e| e.id), Some(id2), "wrong job retrieved for id2");
    assert_eq!(cron.entry(id5).map(|e| e.id), Some(id5), "wrong job retrieved for id5");

    cron.start();

    tokio::time::timeout(ONE_SECOND, async {
        while !ran.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("expected job to run");

    cron.stop();

    // Ensure the entries are in the right order.
    let expected_ids = vec![id2, id4, id5, id1, id3, id0];
    let actual_ids: Vec<_> = cron.entries().into_iter().map(|e| e.id).collect();
    assert_eq!(actual_ids, expected_ids, "jobs not in the right order");

    assert_eq!(cron.entry(id2).map(|e| e.id), Some(id2), "wrong job retrieved for id2");
    assert_eq!(cron.entry(id5).map(|e| e.id), Some(id5), "wrong job retrieved for id5");
}

// Port of TestScheduleAfterRemoval (issue #206): removing one entry must not
// delay the next run of a different, still-scheduled entry.
#[tokio::test]
async fn test_schedule_after_removal() {
    let calls = Arc::new(Mutex::new(0i32));

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);
    let hour_job = cron.schedule(Arc::new(cron_rs::constant_delay::every(chrono::Duration::hours(1))), Arc::new(|| {}));

    let cron_arc = Arc::new(cron);
    let cron_for_job = cron_arc.clone();
    let calls_clone = calls.clone();
    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);
    let done_tx = Arc::new(Mutex::new(Some(done_tx)));

    cron_arc.schedule(
        Arc::new(cron_rs::constant_delay::every(chrono::Duration::seconds(1))),
        Arc::new(move || {
            let mut calls = calls_clone.lock().unwrap();
            match *calls {
                0 => {
                    *calls += 1;
                }
                1 => {
                    *calls += 1;
                    std::thread::sleep(Duration::from_millis(750));
                    cron_for_job.remove(hour_job);
                }
                2 => {
                    *calls += 1;
                    if let Some(tx) = done_tx.lock().unwrap().take() {
                        let _ = tx.try_send(());
                    }
                }
                _ => panic!("unexpected extra call"),
            }
        }),
    );

    cron_arc.start();

    let res = tokio::time::timeout(Duration::from_secs(3), done_rx.recv()).await;
    cron_arc.stop();

    assert!(res.is_ok(), "expected job fires a 3rd time promptly after removal, not delayed to the next full second");
}

// Port of TestMultiThreadedStartAndStop: run() on a background thread, then
// stop() from another; must not deadlock or panic.
#[tokio::test]
async fn test_multi_threaded_start_and_stop() {
    let cron = Arc::new(Cron::with_options(vec![OptionSetter::Parser(second_parser())]));
    let cron_clone = cron.clone();
    let handle = tokio::task::spawn_blocking(move || {
        cron_clone.run();
    });
    tokio::time::sleep(Duration::from_millis(2)).await;
    cron.stop();
    let _ = handle.await;
}
