use cron_rs::parser::*;
use cron_rs::spec::STAR_BIT;
use chrono_tz::Tz;

const MINUTES_BOUNDS: Bounds<'static> = Bounds { min: 0, max: 59, names: None };
const HOURS_BOUNDS: Bounds<'static> = Bounds { min: 0, max: 23, names: None };
const DOM_BOUNDS: Bounds<'static> = Bounds { min: 1, max: 31, names: None };
const MONTHS_BOUNDS: Bounds<'static> = Bounds {
    min: 1,
    max: 12,
    names: Some(&[
        ("jan", 1), ("feb", 2), ("mar", 3), ("apr", 4),
        ("may", 5), ("jun", 6), ("jul", 7), ("aug", 8),
        ("sep", 9), ("oct", 10), ("nov", 11), ("dec", 12),
    ]),
};
const DOW_BOUNDS: Bounds<'static> = Bounds {
    min: 0,
    max: 6,
    names: Some(&[
        ("sun", 0), ("mon", 1), ("tue", 2), ("wed", 3),
        ("thu", 4), ("fri", 5), ("sat", 6),
    ]),
};

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

#[test]
fn test_range() {
    let ranges = vec![
        ("5", 0, 7, 1u64 << 5, ""),
        ("0", 0, 7, 1u64 << 0, ""),
        ("7", 0, 7, 1u64 << 7, ""),
        ("5-5", 0, 7, 1u64 << 5, ""),
        ("5-6", 0, 7, (1u64 << 5) | (1u64 << 6), ""),
        ("5-7", 0, 7, (1u64 << 5) | (1u64 << 6) | (1u64 << 7), ""),
        ("5-6/2", 0, 7, 1u64 << 5, ""),
        ("5-7/2", 0, 7, (1u64 << 5) | (1u64 << 7), ""),
        ("5-7/1", 0, 7, (1u64 << 5) | (1u64 << 6) | (1u64 << 7), ""),
        ("*", 1, 3, (1u64 << 1) | (1u64 << 2) | (1u64 << 3) | STAR_BIT, ""),
        ("*/2", 1, 3, (1u64 << 1) | (1u64 << 3), ""),
        ("5--5", 0, 0, 0, "too many hyphens"),
        ("jan-x", 0, 0, 0, "failed to parse int from"),
        ("2-x", 1, 5, 0, "failed to parse int from"),
        ("*/-12", 0, 0, 0, "negative number"),
        ("*//2", 0, 0, 0, "too many slashes"),
        ("1", 3, 5, 0, "below minimum"),
        ("6", 3, 5, 0, "above maximum"),
        ("5-3", 3, 5, 0, "beyond end of range"),
        ("*/0", 0, 0, 0, "should be a positive number"),
    ];

    for (expr, min, max, expected, err_substr) in ranges {
        let res = get_range(expr, Bounds { min, max, names: None });
        if !err_substr.is_empty() {
            assert!(res.is_err(), "expr {} expected error {}", expr, err_substr);
            let err_msg = format!("{}", res.as_ref().err().unwrap());
            assert!(
                err_msg.contains(err_substr),
                "expr {} expected error containing '{}', got '{}'",
                expr, err_substr, err_msg
            );
        } else {
            assert!(res.is_ok(), "expr {} unexpected error: {:?}", expr, res.err());
            let actual = res.unwrap();
            assert_eq!(actual, expected, "expr {}: expected {}, got {}", expr, expected, actual);
        }
    }
}

#[test]
fn test_field() {
    let fields = vec![
        ("5", 1, 7, 1u64 << 5),
        ("5,6", 1, 7, (1u64 << 5) | (1u64 << 6)),
        ("5,6,7", 1, 7, (1u64 << 5) | (1u64 << 6) | (1u64 << 7)),
        ("1,5-7/2,3", 1, 7, (1u64 << 1) | (1u64 << 5) | (1u64 << 7) | (1u64 << 3)),
    ];

    for (expr, min, max, expected) in fields {
        let actual = get_field(expr, Bounds { min, max, names: None }).unwrap();
        assert_eq!(actual, expected, "expr {}: expected {}, got {}", expr, expected, actual);
    }
}

#[test]
fn test_all() {
    let all_bits = vec![
        (MINUTES_BOUNDS, 0xfffffffffffffffu64),
        (HOURS_BOUNDS, 0xffffffu64),
        (DOM_BOUNDS, 0xfffffffeu64),
        (MONTHS_BOUNDS, 0x1ffeu64),
        (DOW_BOUNDS, 0x7fu64),
    ];

    for (b, expected) in all_bits {
        let actual = all(b);
        assert_eq!(expected | STAR_BIT, actual);
    }
}

#[test]
fn test_bits() {
    let bits = vec![
        (0, 0, 1, 0x1u64),
        (1, 1, 1, 0x2u64),
        (1, 5, 2, 0x2au64),
        (1, 4, 2, 0xau64),
    ];

    for (min, max, step, expected) in bits {
        let actual = get_bits(min, max, step);
        assert_eq!(expected, actual);
    }
}

#[test]
fn test_parse_schedule_errors() {
    let tests = vec![
        ("* 5 j * * *", "failed to parse int from"),
        ("@every Xm", "failed to parse duration"),
        ("@unrecognized", "unrecognized descriptor"),
        ("* * * *", "expected 5 to 6 fields"),
        ("", "empty spec string"),
    ];

    let p = second_parser();
    for (expr, err_substr) in tests {
        let res = p.parse(expr);
        assert!(res.is_err(), "expr {} expected error {}", expr, err_substr);
        let err_msg = format!("{}", res.unwrap_err());
        assert!(
            err_msg.contains(err_substr),
            "expr {} expected error containing '{}', got '{}'",
            expr, err_substr, err_msg
        );
    }
}

#[test]
fn test_parse_schedule() {
    let _tokyo: Tz = "Asia/Tokyo".parse().unwrap();
    let p = second_parser();
    let std_p = standard_parser();

    assert!(p.parse("0 5 * * * *").is_ok());
    assert!(std_p.parse("5 * * * *").is_ok());
    assert!(p.parse("CRON_TZ=UTC 0 5 * * * *").is_ok());
    assert!(std_p.parse("CRON_TZ=UTC 5 * * * *").is_ok());
    assert!(p.parse("CRON_TZ=Asia/Tokyo 0 5 * * * *").is_ok());
    assert!(p.parse("@every 5m").is_ok());
    assert!(p.parse("@midnight").is_ok());
    assert!(p.parse("TZ=UTC @midnight").is_ok());
    assert!(p.parse("TZ=Asia/Tokyo @midnight").is_ok());
    assert!(p.parse("@yearly").is_ok());
    assert!(p.parse("@annually").is_ok());
}

#[test]
fn test_optional_second_schedule() {
    let parser = Parser::new(
        parse_option::SECOND_OPTIONAL
            | parse_option::MINUTE
            | parse_option::HOUR
            | parse_option::DOM
            | parse_option::MONTH
            | parse_option::DOW
            | parse_option::DESCRIPTOR,
    );

    assert!(parser.parse("0 5 * * * *").is_ok());
    assert!(parser.parse("5 5 * * * *").is_ok());
    assert!(parser.parse("5 * * * *").is_ok());
}

#[test]
fn test_normalize_fields() {
    let tests = vec![
        (
            vec!["0", "5", "*", "*", "*", "*"],
            parse_option::SECOND | parse_option::MINUTE | parse_option::HOUR | parse_option::DOM | parse_option::MONTH | parse_option::DOW | parse_option::DESCRIPTOR,
            vec!["0", "5", "*", "*", "*", "*"],
        ),
        (
            vec!["0", "5", "*", "*", "*", "*"],
            parse_option::SECOND_OPTIONAL | parse_option::MINUTE | parse_option::HOUR | parse_option::DOM | parse_option::MONTH | parse_option::DOW | parse_option::DESCRIPTOR,
            vec!["0", "5", "*", "*", "*", "*"],
        ),
        (
            vec!["5", "*", "*", "*", "*"],
            parse_option::SECOND_OPTIONAL | parse_option::MINUTE | parse_option::HOUR | parse_option::DOM | parse_option::MONTH | parse_option::DOW | parse_option::DESCRIPTOR,
            vec!["0", "5", "*", "*", "*", "*"],
        ),
        (
            vec!["5", "15", "*"],
            parse_option::HOUR | parse_option::DOM | parse_option::MONTH,
            vec!["0", "0", "5", "15", "*", "*"],
        ),
        (
            vec!["5", "15", "*", "4"],
            parse_option::HOUR | parse_option::DOM | parse_option::MONTH | parse_option::DOW_OPTIONAL,
            vec!["0", "0", "5", "15", "*", "4"],
        ),
        (
            vec!["5", "15", "*"],
            parse_option::HOUR | parse_option::DOM | parse_option::MONTH | parse_option::DOW_OPTIONAL,
            vec!["0", "0", "5", "15", "*", "*"],
        ),
        (
            vec!["5", "15", "*"],
            parse_option::SECOND_OPTIONAL | parse_option::HOUR | parse_option::DOM | parse_option::MONTH,
            vec!["0", "0", "5", "15", "*", "*"],
        ),
    ];

    for (input, opts, expected) in tests {
        let actual = normalize_fields(&input, opts).expect("unexpected error");
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_normalize_fields_errors() {
    assert!(normalize_fields(&["0", "5", "*", "*", "*", "*"], parse_option::SECOND_OPTIONAL | parse_option::MINUTE | parse_option::HOUR | parse_option::DOM | parse_option::MONTH | parse_option::DOW_OPTIONAL).is_err());
    assert!(normalize_fields(&["0", "5", "*", "*"], parse_option::SECOND_OPTIONAL | parse_option::MINUTE | parse_option::HOUR).is_err());
    assert!(normalize_fields(&[], parse_option::SECOND_OPTIONAL | parse_option::MINUTE | parse_option::HOUR).is_err());
    assert!(normalize_fields(&["*"], parse_option::SECOND_OPTIONAL | parse_option::MINUTE | parse_option::HOUR).is_err());
}

#[test]
fn test_standard_spec_schedule() {
    assert!(parse_standard("5 * * * *").is_ok());
    assert!(parse_standard("@every 5m").is_ok());
    assert!(parse_standard("5 j * * *").is_err());
    assert!(parse_standard("* * * *").is_err());
}

#[test]
fn test_no_descriptor_parser() {
    let p = Parser::new(parse_option::MINUTE | parse_option::HOUR);
    assert!(p.parse("@every 1m").is_err());
}
