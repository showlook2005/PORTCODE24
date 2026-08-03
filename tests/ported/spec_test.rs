use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use cron_rs::parser::parse_standard;

fn parse_time_str(val_str: &str, location: Tz) -> DateTime<Tz> {
    let parts: Vec<&str> = val_str.split_whitespace().collect();
    if parts.is_empty() {
        return location.timestamp_opt(0, 0).unwrap();
    }

    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let clean_str = if weekdays.contains(&parts[0]) {
        parts[1..].join(" ")
    } else {
        parts.join(" ")
    };

    if let Ok(dt) = DateTime::parse_from_str(val_str, "%Y-%m-%dT%H:%M:%S%z") {
        return dt.with_timezone(&location);
    }
    if let Ok(dt) = DateTime::parse_from_str(&clean_str, "%Y-%m-%dT%H:%M:%S%z") {
        return dt.with_timezone(&location);
    }

    let layouts = [
        "%b %d %H:%M %Y",
        "%b %d %H:%M:%S %Y",
        "%b %e %H:%M %Y",
        "%b %e %H:%M:%S %Y",
    ];

    for layout in &layouts {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&clean_str, layout) {
            return location.from_utc_datetime(&dt);
        }
    }

    panic!("could not parse time value {}", val_str);
}

fn get_time(value: &str) -> DateTime<Tz> {
    if value.is_empty() {
        return Tz::UTC.timestamp_opt(0, 0).unwrap();
    }

    let mut location = Tz::UTC;
    let mut val_str = value;

    if value.starts_with("TZ=") {
        let parts: Vec<&str> = value.split_whitespace().collect();
        let tz_name = &parts[0]["TZ=".len()..];
        location = tz_name.parse::<Tz>().expect("could not parse location");
        val_str = parts[1];
    }

    parse_time_str(val_str, location)
}

fn get_time_tz(value: &str) -> DateTime<Tz> {
    if value.is_empty() {
        return Tz::UTC.timestamp_opt(0, 0).unwrap();
    }
    if value.contains("+0530") {
        let kolkata: Tz = "Asia/Kolkata".parse().unwrap();
        let clean = value.replace("+0530", "");
        let ndt = NaiveDateTime::parse_from_str(&clean, "%Y-%m-%dT%H:%M:%S").unwrap();
        return kolkata.from_local_datetime(&ndt).unwrap();
    }
    parse_time_str(value, Tz::UTC)
}

#[test]
fn test_activation() {
    let tests = vec![
        ("Mon Jul 9 15:00 2012", "0/15 * * * *", true),
        ("Mon Jul 9 15:45 2012", "0/15 * * * *", true),
        ("Mon Jul 9 15:40 2012", "0/15 * * * *", false),
        ("Mon Jul 9 15:05 2012", "5/15 * * * *", true),
        ("Mon Jul 9 15:20 2012", "5/15 * * * *", true),
        ("Mon Jul 9 15:50 2012", "5/15 * * * *", true),
        ("Sun Jul 15 15:00 2012", "0/15 * * Jul *", true),
        ("Sun Jul 15 15:00 2012", "0/15 * * Jun *", false),
        ("Sun Jul 15 08:30 2012", "30 08 ? Jul Sun", true),
        ("Sun Jul 15 08:30 2012", "30 08 15 Jul ?", true),
        ("Mon Jul 16 08:30 2012", "30 08 ? Jul Sun", false),
        ("Mon Jul 16 08:30 2012", "30 08 15 Jul ?", false),
        ("Mon Jul 9 15:00 2012", "@hourly", true),
        ("Mon Jul 9 15:04 2012", "@hourly", false),
        ("Mon Jul 9 15:00 2012", "@daily", false),
        ("Mon Jul 9 00:00 2012", "@daily", true),
        ("Mon Jul 9 00:00 2012", "@weekly", false),
        ("Sun Jul 8 00:00 2012", "@weekly", true),
        ("Sun Jul 8 01:00 2012", "@weekly", false),
        ("Sun Jul 8 00:00 2012", "@monthly", false),
        ("Sun Jul 1 00:00 2012", "@monthly", true),
        ("Sun Jul 15 00:00 2012", "* * 1,15 * Sun", true),
        ("Fri Jun 15 00:00 2012", "* * 1,15 * Sun", true),
        ("Wed Aug 1 00:00 2012", "* * 1,15 * Sun", true),
        ("Sun Jul 15 00:00 2012", "* * */10 * Sun", true),
        ("Sun Jul 15 00:00 2012", "* * * * Mon", false),
        ("Mon Jul 9 00:00 2012", "* * 1,15 * *", false),
        ("Sun Jul 15 00:00 2012", "* * 1,15 * *", true),
        ("Sun Jul 15 00:00 2012", "* * */2 * Sun", true),
    ];

    for (time_str, spec, expected) in tests {
        let sched = parse_standard(spec).expect("failed to parse spec");
        let t = get_time(time_str);
        let actual = sched.next(t - chrono::Duration::seconds(1));
        let exp_time = get_time(time_str);
        let is_match = actual.map_or(false, |act| act.timestamp() == exp_time.timestamp());
        assert_eq!(
            is_match, expected,
            "Fail evaluating {} on {}: expected match {}",
            spec, time_str, expected
        );
    }
}

#[test]
fn test_next() {
    let runs = vec![
        ("Mon Jul 9 14:45 2012", "0 0/15 * * * *", "Mon Jul 9 15:00 2012"),
        ("Mon Jul 9 14:59 2012", "0 0/15 * * * *", "Mon Jul 9 15:00 2012"),
        ("Mon Jul 9 14:59:59 2012", "0 0/15 * * * *", "Mon Jul 9 15:00 2012"),
        ("Mon Jul 9 15:45 2012", "0 20-35/15 * * * *", "Mon Jul 9 16:20 2012"),
        ("Mon Jul 9 23:46 2012", "0 */15 * * * *", "Tue Jul 10 00:00 2012"),
        ("Mon Jul 9 23:45 2012", "0 20-35/15 * * * *", "Tue Jul 10 00:20 2012"),
        ("Mon Jul 9 23:35:51 2012", "15/35 20-35/15 * * * *", "Tue Jul 10 00:20:15 2012"),
        ("Mon Jul 9 23:35:51 2012", "15/35 20-35/15 1/2 * * *", "Tue Jul 10 01:20:15 2012"),
        ("Mon Jul 9 23:35:51 2012", "15/35 20-35/15 10-12 * * *", "Tue Jul 10 10:20:15 2012"),
        ("Mon Jul 9 23:35:51 2012", "15/35 20-35/15 1/2 */2 * *", "Thu Jul 11 01:20:15 2012"),
        ("Mon Jul 9 23:35:51 2012", "15/35 20-35/15 * 9-20 * *", "Wed Jul 10 00:20:15 2012"),
        ("Mon Jul 9 23:35:51 2012", "15/35 20-35/15 * 9-20 Jul *", "Wed Jul 10 00:20:15 2012"),
        ("Mon Jul 9 23:35 2012", "0 0 0 9 Apr-Oct ?", "Thu Aug 9 00:00 2012"),
        ("Mon Jul 9 23:35 2012", "0 0 0 */5 Apr,Aug,Oct Mon", "Tue Aug 1 00:00 2012"),
        ("Mon Jul 9 23:35 2012", "0 0 0 */5 Oct Mon", "Mon Oct 1 00:00 2012"),
        ("Mon Jul 9 23:35 2012", "0 0 0 * Feb Mon", "Mon Feb 4 00:00 2013"),
        ("Mon Jul 9 23:35 2012", "0 0 0 * Feb Mon/2", "Fri Feb 1 00:00 2013"),
        ("Mon Dec 31 23:59:45 2012", "0 * * * * *", "Tue Jan 1 00:00:00 2013"),
        ("Mon Jul 9 23:35 2012", "0 0 0 29 Feb ?", "Mon Feb 29 00:00 2016"),
        ("2012-03-11T00:00:00-0500", "TZ=America/New_York 0 30 2 11 Mar ?", "2013-03-11T02:30:00-0400"),
        ("2012-03-11T00:00:00-0500", "TZ=America/New_York 0 0 * * * ?", "2012-03-11T01:00:00-0500"),
        ("2012-03-11T01:00:00-0500", "TZ=America/New_York 0 0 * * * ?", "2012-03-11T03:00:00-0400"),
        ("2012-03-11T03:00:00-0400", "TZ=America/New_York 0 0 * * * ?", "2012-03-11T04:00:00-0400"),
        ("2012-03-11T04:00:00-0400", "TZ=America/New_York 0 0 * * * ?", "2012-03-11T05:00:00-0400"),
        ("2012-03-11T00:00:00-0500", "CRON_TZ=America/New_York 0 0 * * * ?", "2012-03-11T01:00:00-0500"),
        ("2012-03-11T01:00:00-0500", "CRON_TZ=America/New_York 0 0 * * * ?", "2012-03-11T03:00:00-0400"),
        ("2012-03-11T03:00:00-0400", "CRON_TZ=America/New_York 0 0 * * * ?", "2012-03-11T04:00:00-0400"),
        ("2012-03-11T04:00:00-0400", "CRON_TZ=America/New_York 0 0 * * * ?", "2012-03-11T05:00:00-0400"),
        ("2012-03-11T00:00:00-0500", "TZ=America/New_York 0 0 1 * * ?", "2012-03-11T01:00:00-0500"),
        ("2012-03-11T01:00:00-0500", "TZ=America/New_York 0 0 1 * * ?", "2012-03-12T01:00:00-0400"),
        ("2012-03-11T00:00:00-0500", "TZ=America/New_York 0 0 2 * * ?", "2012-03-12T02:00:00-0400"),
        ("2012-11-04T00:00:00-0400", "TZ=America/New_York 0 30 2 04 Nov ?", "2012-11-04T02:30:00-0500"),
        ("2012-11-04T01:45:00-0400", "TZ=America/New_York 0 30 1 04 Nov ?", "2012-11-04T01:30:00-0500"),
        ("2012-11-04T00:00:00-0400", "TZ=America/New_York 0 0 * * * ?", "2012-11-04T01:00:00-0400"),
        ("2012-11-04T01:00:00-0400", "TZ=America/New_York 0 0 * * * ?", "2012-11-04T01:00:00-0500"),
        ("2012-11-04T01:00:00-0500", "TZ=America/New_York 0 0 * * * ?", "2012-11-04T02:00:00-0500"),
        ("2012-11-04T00:00:00-0400", "TZ=America/New_York 0 0 1 * * ?", "2012-11-04T01:00:00-0400"),
        ("2012-11-04T01:00:00-0400", "TZ=America/New_York 0 0 1 * * ?", "2012-11-04T01:00:00-0500"),
        ("2012-11-04T01:00:00-0500", "TZ=America/New_York 0 0 1 * * ?", "2012-11-05T01:00:00-0500"),
        ("2012-11-04T00:00:00-0400", "TZ=America/New_York 0 0 2 * * ?", "2012-11-04T02:00:00-0500"),
        ("2012-11-04T02:00:00-0500", "TZ=America/New_York 0 0 2 * * ?", "2012-11-05T02:00:00-0500"),
        ("2012-11-04T00:00:00-0400", "TZ=America/New_York 0 0 3 * * ?", "2012-11-04T03:00:00-0500"),
        ("2012-11-04T03:00:00-0500", "TZ=America/New_York 0 0 3 * * ?", "2012-11-05T03:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T00:00:00-0400", "0 0 * * * ?", "2012-11-04T01:00:00-0400"),
        ("TZ=America/New_York 2012-11-04T01:00:00-0400", "0 0 * * * ?", "2012-11-04T01:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T01:00:00-0500", "0 0 * * * ?", "2012-11-04T02:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T00:00:00-0400", "0 0 1 * * ?", "2012-11-04T01:00:00-0400"),
        ("TZ=America/New_York 2012-11-04T01:00:00-0400", "0 0 1 * * ?", "2012-11-04T01:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T01:00:00-0500", "0 0 1 * * ?", "2012-11-05T01:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T00:00:00-0400", "0 0 2 * * ?", "2012-11-04T02:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T02:00:00-0500", "0 0 2 * * ?", "2012-11-05T02:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T00:00:00-0400", "0 0 3 * * ?", "2012-11-04T03:00:00-0500"),
        ("TZ=America/New_York 2012-11-04T03:00:00-0500", "0 0 3 * * ?", "2012-11-05T03:00:00-0500"),
        ("Mon Jul 9 23:35 2012", "0 0 0 30 Feb ?", ""),
        ("Mon Jul 9 23:35 2012", "0 0 0 31 Apr ?", ""),
        ("TZ=America/New_York 2012-11-04T00:00:00-0400", "0 0 3 3 * ?", "2012-12-03T03:00:00-0500"),
        ("2018-10-17T05:00:00-0400", "TZ=America/Sao_Paulo 0 0 9 10 * ?", "2018-11-10T06:00:00-0500"),
        ("2018-02-14T05:00:00-0500", "TZ=America/Sao_Paulo 0 0 9 22 * ?", "2018-02-22T07:00:00-0500"),
    ];

    let parser = cron_rs::parser::Parser::new(
        cron_rs::parser::parse_option::SECOND
            | cron_rs::parser::parse_option::MINUTE
            | cron_rs::parser::parse_option::HOUR
            | cron_rs::parser::parse_option::DOM
            | cron_rs::parser::parse_option::MONTH
            | cron_rs::parser::parse_option::DOW_OPTIONAL
            | cron_rs::parser::parse_option::DESCRIPTOR,
    );

    for (time_str, spec, expected_str) in runs {
        let sched = parser.parse(spec).expect("failed to parse spec");
        let t = get_time(time_str);
        let actual = sched.next(t);
        if expected_str.is_empty() {
            assert!(actual.is_none(), "expected None for spec {} on {}", spec, time_str);
        } else {
            let expected = get_time(expected_str);
            assert!(
                actual.is_some(),
                "expected next time for spec {} on {}, got None",
                spec, time_str
            );
            let act = actual.unwrap();
            assert_eq!(
                act.timestamp(), expected.timestamp(),
                "spec {}, time {}: (expected) {} != {} (actual)",
                spec, time_str, expected, act
            );
        }
    }
}

#[test]
fn test_errors() {
    let invalid_specs = vec![
        "xyz",
        "60 0 * * *",
        "0 60 * * *",
        "0 0 * * XYZ",
    ];
    for spec in invalid_specs {
        let res = parse_standard(spec);
        assert!(res.is_err(), "expected error parsing: {}", spec);
    }
}

#[test]
fn test_next_with_tz() {
    let runs = vec![
        ("2016-01-03T13:09:03+0530", "14 14 * * *", "2016-01-03T14:14:00+0530"),
        ("2016-01-03T04:09:03+0530", "14 14 * * ?", "2016-01-03T14:14:00+0530"),
        ("2016-01-03T14:09:03+0530", "14 14 * * *", "2016-01-03T14:14:00+0530"),
        ("2016-01-03T14:00:00+0530", "14 14 * * ?", "2016-01-03T14:14:00+0530"),
    ];

    for (time_str, spec, expected_str) in runs {
        let sched = parse_standard(spec).expect("failed to parse spec");
        let t = get_time_tz(time_str);
        let actual = sched.next(t).expect("expected next time");
        let expected = get_time_tz(expected_str);
        assert_eq!(
            actual.timestamp(), expected.timestamp(),
            "spec {}, time {}: (expected) {} != {} (actual)",
            spec, time_str, expected, actual
        );
    }
}

#[test]
fn test_slash_0_no_hang() {
    let schedule = "TZ=America/New_York 15/0 * * * *";
    let res = parse_standard(schedule);
    assert!(res.is_err(), "expected error on 0 increment");
}
