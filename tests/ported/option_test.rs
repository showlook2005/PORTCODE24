use cron_rs::cron::{Cron, OptionSetter};
use cron_rs::logger::Logger;
use cron_rs::parser::{parse_option, Parser};
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ONE_SECOND: Duration = Duration::from_millis(1050);

// Port of TestWithLocation.
#[test]
fn test_with_location() {
    let cron = Cron::with_options(vec![OptionSetter::Location(chrono_tz::Tz::UTC)]);
    assert_eq!(cron.location(), chrono_tz::Tz::UTC, "expected UTC, got {:?}", cron.location());
}

// Port of TestWithParser. Go asserts the private `parser` field was set to
// the exact provided value; Rust's `Cron` keeps that field private (no such
// getter is part of the public API), so this checks the same thing
// behaviorally: a parser restricted to DOW-only fields must actually be used
// to parse `AddFunc`'s spec, which the default (multi-field) parser could not.
#[test]
fn test_with_parser() {
    let dow_only_parser = Parser::new(parse_option::DOW);
    let cron = Cron::with_options(vec![OptionSetter::Parser(dow_only_parser)]);

    // A single DOW field ("1" = Monday) only parses successfully if the
    // custom Dow-only parser is the one actually in use.
    let result = cron.add_func("1", || {});
    assert!(result.is_ok(), "expected provided (Dow-only) parser to be used, got: {:?}", result.err());
}

// Port of TestWithVerboseLogger. Go asserts on the private `printfLogger`
// field; here we install a custom Logger (mirroring `SyncLogger` used
// elsewhere in this crate) and assert on its observable output instead,
// since Rust's `Cron` doesn't expose its logger field for downcasting.
struct SyncLogger {
    logs: Arc<Mutex<Vec<String>>>,
}

impl Logger for SyncLogger {
    fn info(&self, msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {
        self.logs.lock().unwrap().push(format!("{},", msg));
    }
    fn error(&self, err: &dyn std::error::Error, msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {
        self.logs.lock().unwrap().push(format!("{},: {}", msg, err));
    }
}

#[tokio::test]
async fn test_with_verbose_logger() {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logger: Arc<dyn Logger> = Arc::new(SyncLogger { logs: logs.clone() });

    let cron = Cron::with_options(vec![OptionSetter::Logger(logger)]);
    let _ = cron.add_func("@every 1s", || {});
    cron.start();
    tokio::time::sleep(ONE_SECOND).await;
    cron.stop();

    let out = logs.lock().unwrap().join("\n");
    assert!(out.contains("schedule,"), "expected to see some actions, got: {}", out);
    assert!(out.contains("run,"), "expected to see some actions, got: {}", out);
}
