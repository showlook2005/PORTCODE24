use cron_rs::chain::{recover, Chain};
use cron_rs::cron::*;
use cron_rs::logger::{DiscardLogger, Logger};
use cron_rs::parser::{parse_option, Parser};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

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
async fn test_soak_concurrency_and_panic_recovery() {
    let discard: Arc<dyn Logger> = Arc::new(DiscardLogger);
    let cron = Arc::new(Cron::with_options(vec![
        OptionSetter::Parser(second_parser()),
        OptionSetter::Chain(Chain::new(vec![recover(discard)])),
    ]));

    cron.start();

    let counter = Arc::new(AtomicUsize::new(0));

    // Spawn 20 tasks concurrently adding, removing, starting, and stopping
    let mut handles = Vec::new();

    for i in 0..20 {
        let cron_clone = cron.clone();
        let counter_clone = counter.clone();

        handles.push(tokio::spawn(async move {
            for j in 0..50 {
                if (i + j) % 7 == 0 {
                    // Panicking job to test recovery wrapper
                    let id = cron_clone
                        .add_func("* * * * * ?", || {
                            panic!("soak simulated panic");
                        })
                        .unwrap();
                    sleep(Duration::from_millis(5)).await;
                    cron_clone.remove(id);
                } else {
                    let c = counter_clone.clone();
                    let id = cron_clone
                        .add_func("* * * * * ?", move || {
                            c.fetch_add(1, Ordering::SeqCst);
                        })
                        .unwrap();
                    sleep(Duration::from_millis(5)).await;
                    cron_clone.remove(id);
                }
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let rx = cron.stop();
    let _ = rx.await;

    // Verify scheduler state remains clean and entries are empty after removal
    assert_eq!(cron.entries().len(), 0);
}
