use crate::constant_delay::every;
use crate::schedule::Schedule;
use crate::spec::{SpecSchedule, STAR_BIT};
use chrono::Duration;
use chrono_tz::Tz;
use std::sync::Arc;
use thiserror::Error;

pub mod parse_option {
    pub const SECOND: u32 = 1 << 0;
    pub const SECOND_OPTIONAL: u32 = 1 << 1;
    pub const MINUTE: u32 = 1 << 2;
    pub const HOUR: u32 = 1 << 3;
    pub const DOM: u32 = 1 << 4;
    pub const MONTH: u32 = 1 << 5;
    pub const DOW: u32 = 1 << 6;
    pub const DOW_OPTIONAL: u32 = 1 << 7;
    pub const DESCRIPTOR: u32 = 1 << 8;
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ParseError {
    #[error("empty spec string")]
    EmptySpec,
    #[error("{0}")]
    Message(String),
}

#[derive(Clone, Copy)]
pub struct Bounds<'a> {
    pub min: u32,
    pub max: u32,
    pub names: Option<&'a [(&'static str, u32)]>,
}

const SECONDS_BOUNDS: Bounds<'static> = Bounds { min: 0, max: 59, names: None };
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

const PLACES: [u32; 6] = [
    parse_option::SECOND,
    parse_option::MINUTE,
    parse_option::HOUR,
    parse_option::DOM,
    parse_option::MONTH,
    parse_option::DOW,
];

const DEFAULTS: [&str; 6] = ["0", "0", "0", "*", "*", "*"];

pub fn get_bits(min: u32, max: u32, step: u32) -> u64 {
    let mut bits: u64 = 0;
    if step == 1 {
        for i in min..=max {
            bits |= 1u64 << i;
        }
        return bits;
    }
    let mut i = min;
    while i <= max {
        bits |= 1u64 << i;
        i += step;
    }
    bits
}

pub fn all(b: Bounds) -> u64 {
    get_bits(b.min, b.max, 1) | STAR_BIT
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parser {
    pub options: u32,
}

impl Parser {
    pub fn new(options: u32) -> Self {
        let mut optionals = 0;
        if options & parse_option::DOW_OPTIONAL > 0 {
            optionals += 1;
        }
        if options & parse_option::SECOND_OPTIONAL > 0 {
            optionals += 1;
        }
        if optionals > 1 {
            panic!("multiple optionals may not be configured");
        }
        Parser { options }
    }

    pub fn parse(&self, spec: &str) -> Result<Arc<dyn Schedule>, ParseError> {
        let mut spec = spec.trim();
        if spec.is_empty() {
            return Err(ParseError::EmptySpec);
        }

        let mut loc: Option<Tz> = None;

        if spec.starts_with("TZ=") || spec.starts_with("CRON_TZ=") {
            let eq_pos = spec.find('=').unwrap();
            let space_pos = match spec.find(' ') {
                Some(idx) => idx,
                None => return Err(ParseError::Message(format!("provided bad location {}", &spec[eq_pos + 1..]))),
            };
            let loc_str = &spec[eq_pos + 1..space_pos];
            let parsed_loc = match loc_str.parse::<Tz>() {
                Ok(t) => t,
                Err(_) => return Err(ParseError::Message(format!("provided bad location {}", loc_str))),
            };
            loc = Some(parsed_loc);
            spec = spec[space_pos..].trim();
        }

        if spec.starts_with('@') {
            if self.options & parse_option::DESCRIPTOR == 0 {
                return Err(ParseError::Message(format!("parser does not accept descriptors: {}", spec)));
            }
            return parse_descriptor(spec, loc);
        }

        let fields_vec: Vec<&str> = spec.split_whitespace().collect();
        let normalized = normalize_fields(&fields_vec, self.options)?;

        let second = get_field(&normalized[0], SECONDS_BOUNDS)?;
        let minute = get_field(&normalized[1], MINUTES_BOUNDS)?;
        let hour = get_field(&normalized[2], HOURS_BOUNDS)?;
        let dom = get_field(&normalized[3], DOM_BOUNDS)?;
        let month = get_field(&normalized[4], MONTHS_BOUNDS)?;
        let dow = get_field(&normalized[5], DOW_BOUNDS)?;

        Ok(Arc::new(SpecSchedule {
            second,
            minute,
            hour,
            dom,
            month,
            dow,
            location: loc,
        }))
    }
}

pub fn standard_parser() -> Parser {
    Parser::new(
        parse_option::MINUTE
            | parse_option::HOUR
            | parse_option::DOM
            | parse_option::MONTH
            | parse_option::DOW
            | parse_option::DESCRIPTOR,
    )
}

pub fn parse_standard(standard_spec: &str) -> Result<Arc<dyn Schedule>, ParseError> {
    standard_parser().parse(standard_spec)
}

pub fn normalize_fields(fields: &[&str], mut options: u32) -> Result<Vec<String>, ParseError> {
    let mut optionals = 0;
    if options & parse_option::SECOND_OPTIONAL > 0 {
        options |= parse_option::SECOND;
        optionals += 1;
    }
    if options & parse_option::DOW_OPTIONAL > 0 {
        options |= parse_option::DOW;
        optionals += 1;
    }
    if optionals > 1 {
        return Err(ParseError::Message("multiple optionals may not be configured".to_string()));
    }

    let mut max = 0;
    for place in PLACES {
        if options & place > 0 {
            max += 1;
        }
    }
    let min = max - optionals;

    let count = fields.len();
    if count < min || count > max {
        if min == max {
            return Err(ParseError::Message(format!(
                "expected exactly {} fields, found {}: {:?}",
                min, count, fields
            )));
        }
        return Err(ParseError::Message(format!(
            "expected {} to {} fields, found {}: {:?}",
            min, max, count, fields
        )));
    }

    let mut working_fields: Vec<String> = fields.iter().map(|s| s.to_string()).collect();

    if min < max && count == min {
        if options & parse_option::DOW_OPTIONAL > 0 {
            working_fields.push(DEFAULTS[5].to_string());
        } else if options & parse_option::SECOND_OPTIONAL > 0 {
            working_fields.insert(0, DEFAULTS[0].to_string());
        } else {
            return Err(ParseError::Message("unknown optional field".to_string()));
        }
    }

    let mut expanded: Vec<String> = DEFAULTS.iter().map(|s| s.to_string()).collect();
    let mut n = 0;
    for (i, place) in PLACES.iter().enumerate() {
        if options & place > 0 {
            expanded[i] = working_fields[n].clone();
            n += 1;
        }
    }

    Ok(expanded)
}

pub fn get_field(field: &str, r: Bounds) -> Result<u64, ParseError> {
    let mut bits: u64 = 0;
    let ranges: Vec<&str> = field.split(',').collect();
    for expr in ranges {
        let bit = get_range(expr, r)?;
        bits |= bit;
    }
    Ok(bits)
}

pub fn get_range(expr: &str, r: Bounds) -> Result<u64, ParseError> {
    let range_and_step: Vec<&str> = expr.split('/').collect();
    let low_and_high: Vec<&str> = range_and_step[0].split('-').collect();
    let single_digit = low_and_high.len() == 1;

    let mut extra: u64 = 0;
    let start: u32;
    let mut end: u32;

    if low_and_high[0] == "*" || low_and_high[0] == "?" {
        start = r.min;
        end = r.max;
        extra = STAR_BIT;
    } else {
        start = parse_int_or_name(low_and_high[0], r.names)?;
        match low_and_high.len() {
            1 => end = start,
            2 => end = parse_int_or_name(low_and_high[1], r.names)?,
            _ => return Err(ParseError::Message(format!("too many hyphens: {}", expr))),
        }
    }

    let step: u32;
    match range_and_step.len() {
        1 => step = 1,
        2 => {
            step = must_parse_int(range_and_step[1])?;
            if single_digit {
                end = r.max;
            }
            if step > 1 {
                extra = 0;
            }
        }
        _ => return Err(ParseError::Message(format!("too many slashes: {}", expr))),
    }

    if start < r.min {
        return Err(ParseError::Message(format!(
            "beginning of range ({}) below minimum ({}): {}",
            start, r.min, expr
        )));
    }
    if end > r.max {
        return Err(ParseError::Message(format!(
            "end of range ({}) above maximum ({}): {}",
            end, r.max, expr
        )));
    }
    if start > end {
        return Err(ParseError::Message(format!(
            "beginning of range ({}) beyond end of range ({}): {}",
            start, end, expr
        )));
    }
    if step == 0 {
        return Err(ParseError::Message(format!(
            "step of range should be a positive number: {}",
            expr
        )));
    }

    Ok(get_bits(start, end, step) | extra)
}

fn parse_int_or_name(expr: &str, names: Option<&[(&'static str, u32)]>) -> Result<u32, ParseError> {
    if let Some(list) = names {
        let lower = expr.to_lowercase();
        for &(name, val) in list {
            if name == lower {
                return Ok(val);
            }
        }
    }
    must_parse_int(expr)
}

fn must_parse_int(expr: &str) -> Result<u32, ParseError> {
    let num = expr.parse::<i32>().map_err(|e| {
        ParseError::Message(format!("failed to parse int from {}: {}", expr, e))
    })?;
    if num < 0 {
        return Err(ParseError::Message(format!(
            "negative number ({}) not allowed: {}",
            num, expr
        )));
    }
    Ok(num as u32)
}

pub fn parse_descriptor(descriptor: &str, loc: Option<Tz>) -> Result<Arc<dyn Schedule>, ParseError> {
    match descriptor {
        "@yearly" | "@annually" => Ok(Arc::new(SpecSchedule {
            second: 1 << SECONDS_BOUNDS.min,
            minute: 1 << MINUTES_BOUNDS.min,
            hour: 1 << HOURS_BOUNDS.min,
            dom: 1 << DOM_BOUNDS.min,
            month: 1 << MONTHS_BOUNDS.min,
            dow: all(DOW_BOUNDS),
            location: loc,
        })),
        "@monthly" => Ok(Arc::new(SpecSchedule {
            second: 1 << SECONDS_BOUNDS.min,
            minute: 1 << MINUTES_BOUNDS.min,
            hour: 1 << HOURS_BOUNDS.min,
            dom: 1 << DOM_BOUNDS.min,
            month: all(MONTHS_BOUNDS),
            dow: all(DOW_BOUNDS),
            location: loc,
        })),
        "@weekly" => Ok(Arc::new(SpecSchedule {
            second: 1 << SECONDS_BOUNDS.min,
            minute: 1 << MINUTES_BOUNDS.min,
            hour: 1 << HOURS_BOUNDS.min,
            dom: all(DOM_BOUNDS),
            month: all(MONTHS_BOUNDS),
            dow: 1 << DOW_BOUNDS.min,
            location: loc,
        })),
        "@daily" | "@midnight" => Ok(Arc::new(SpecSchedule {
            second: 1 << SECONDS_BOUNDS.min,
            minute: 1 << MINUTES_BOUNDS.min,
            hour: 1 << HOURS_BOUNDS.min,
            dom: all(DOM_BOUNDS),
            month: all(MONTHS_BOUNDS),
            dow: all(DOW_BOUNDS),
            location: loc,
        })),
        "@hourly" => Ok(Arc::new(SpecSchedule {
            second: 1 << SECONDS_BOUNDS.min,
            minute: 1 << MINUTES_BOUNDS.min,
            hour: all(HOURS_BOUNDS),
            dom: all(DOM_BOUNDS),
            month: all(MONTHS_BOUNDS),
            dow: all(DOW_BOUNDS),
            location: loc,
        })),
        _ => {
            const EVERY: &str = "@every ";
            if descriptor.starts_with(EVERY) {
                let duration_str = &descriptor[EVERY.len()..];
                let duration = parse_duration(duration_str)?;
                return Ok(Arc::new(every(duration)));
            }
            Err(ParseError::Message(format!("unrecognized descriptor: {}", descriptor)))
        }
    }
}

pub fn parse_duration(s: &str) -> Result<Duration, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Message(format!("failed to parse duration {}: empty", s)));
    }
    let mut total_nanos: i64 = 0;
    let mut current_num: i64 = 0;
    let mut has_digit = false;
    let mut chars = s.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_digit = true;
            current_num = current_num * 10 + (c as i64 - '0' as i64);
            chars.next();
        } else {
            if !has_digit {
                return Err(ParseError::Message(format!("failed to parse duration {}: invalid format", s)));
            }
            let mut unit = String::new();
            while let Some(&u) = chars.peek() {
                if !u.is_ascii_digit() {
                    unit.push(u);
                    chars.next();
                } else {
                    break;
                }
            }
            match unit.as_str() {
                "ns" => total_nanos += current_num,
                "us" | "µs" => total_nanos += current_num * 1_000,
                "ms" => total_nanos += current_num * 1_000_000,
                "s" => total_nanos += current_num * 1_000_000_000,
                "m" => total_nanos += current_num * 60 * 1_000_000_000,
                "h" => total_nanos += current_num * 3600 * 1_000_000_000,
                _ => return Err(ParseError::Message(format!("failed to parse duration {}: unknown unit {}", s, unit))),
            }
            current_num = 0;
            has_digit = false;
        }
    }

    if has_digit {
        return Err(ParseError::Message(format!("failed to parse duration {}: missing unit", s)));
    }

    Ok(Duration::nanoseconds(total_nanos))
}
