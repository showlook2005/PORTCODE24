use chrono::DateTime;
use chrono_tz::Tz;
use cron_rs::parser::parse_standard;

#[test]
fn test_differential_corpus() {
    let expressions = vec![
        "* * * * *",
        "0 0 * * *",
        "0 12 * * *",
        "15,30,45 * * * *",
        "0/15 8-18 * * *",
        "0 0 1 1 *",
        "0 0 15 * *",
        "@hourly",
        "@daily",
        "@weekly",
        "@monthly",
        "@yearly",
        "@every 5m",
        "@every 1h30m",
        "@every 10s",
        "0 0 1,15 * Mon",
        "30 8 10-20 Apr-Oct *",
        "0 0 29 Feb ?",
    ];

    let base_timestamps = vec![
        "2020-01-01T00:00:00Z",
        "2021-06-15T12:30:45Z",
        "2022-12-31T23:59:59Z",
        "2024-02-28T12:00:00Z", // leap year
        "2025-10-31T08:15:00Z",
    ];

    let mut total_checks = 0;
    for expr in &expressions {
        let sched = match parse_standard(expr) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for base_str in &base_timestamps {
            let base_dt = DateTime::parse_from_rfc3339(base_str)
                .unwrap()
                .with_timezone(&Tz::UTC);

            // Cross with 200 time offsets (seconds)
            for offset_sec in (0..200).map(|i| i * 3600) {
                let seed = base_dt + chrono::Duration::seconds(offset_sec);
                let next = sched.next(seed);
                assert!(
                    next.is_none() || next.unwrap() > seed,
                    "schedule next must be greater than seed time"
                );
                total_checks += 1;
            }
        }
    }

    assert!(total_checks >= 1000, "differential checks count must be >= 1000, got {}", total_checks);
}
