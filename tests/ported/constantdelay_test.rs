use chrono::{DateTime, Duration, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use cron_rs::constant_delay::every;
use cron_rs::schedule::Schedule;

fn get_time(value: &str) -> DateTime<Tz> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.is_empty() {
        return Tz::UTC.timestamp_opt(0, 0).unwrap();
    }

    // Skip weekday prefix if present (e.g. "Mon ", "Thu ")
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let (val_str, has_weekday) = if weekdays.contains(&parts[0]) {
        (parts[1..].join(" "), true)
    } else {
        (parts.join(" "), false)
    };

    let layouts = [
        "%b %d %H:%M %Y",
        "%b %d %H:%M:%S %Y",
        "%b %e %H:%M %Y",
        "%b %e %H:%M:%S %Y",
        "%a %b %d %H:%M %Y",
        "%a %b %d %H:%M:%S %Y",
        "%Y-%m-%dT%H:%M:%S%z",
    ];

    for layout in &layouts {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&val_str, layout) {
            return Tz::UTC.from_utc_datetime(&dt);
        }
        if let Ok(dt) = DateTime::parse_from_str(&val_str, layout) {
            return dt.with_timezone(&Tz::UTC);
        }
    }

    // Fallback if not stripped
    if has_weekday {
        let full = parts.join(" ");
        for layout in &layouts {
            if let Ok(dt) = NaiveDateTime::parse_from_str(&full, layout) {
                return Tz::UTC.from_utc_datetime(&dt);
            }
        }
    }

    panic!("could not parse time value {}", value);
}

#[test]
fn test_constant_delay_next() {
    let tests = vec![
        ("Mon Jul 9 14:45 2012", Duration::minutes(15) + Duration::nanoseconds(50), "Mon Jul 9 15:00 2012"),
        ("Mon Jul 9 14:59 2012", Duration::minutes(15), "Mon Jul 9 15:14 2012"),
        ("Mon Jul 9 14:59:59 2012", Duration::minutes(15), "Mon Jul 9 15:14:59 2012"),
        ("Mon Jul 9 15:45 2012", Duration::minutes(35), "Mon Jul 9 16:20 2012"),
        ("Mon Jul 9 23:46 2012", Duration::minutes(14), "Tue Jul 10 00:00 2012"),
        ("Mon Jul 9 23:45 2012", Duration::minutes(35), "Tue Jul 10 00:20 2012"),
        ("Mon Jul 9 23:35:51 2012", Duration::minutes(44) + Duration::seconds(24), "Tue Jul 10 00:20:15 2012"),
        ("Mon Jul 9 23:35:51 2012", Duration::hours(25) + Duration::minutes(44) + Duration::seconds(24), "Thu Jul 11 01:20:15 2012"),
        ("Mon Jul 9 23:35 2012", Duration::hours(91 * 24) + Duration::minutes(25), "Thu Oct 9 00:00 2012"),
        ("Mon Dec 31 23:59:45 2012", Duration::seconds(15), "Tue Jan 1 00:00:00 2013"),
        ("Mon Jul 9 14:45 2012", Duration::minutes(15) + Duration::nanoseconds(50), "Mon Jul 9 15:00 2012"),
        ("Mon Jul 9 14:45:00 2012", Duration::milliseconds(15), "Mon Jul 9 14:45:01 2012"),
        ("Mon Jul 9 14:45:00 2012", Duration::minutes(15), "Mon Jul 9 15:00 2012"),
    ];

    for (time_str, delay, expected_str) in tests {
        let actual = every(delay).next(get_time(time_str)).unwrap();
        let expected = get_time(expected_str);
        assert_eq!(
            actual.timestamp(), expected.timestamp(),
            "{}, {:?}: (expected) {} != {} (actual)",
            time_str, delay, expected, actual
        );
    }
}
