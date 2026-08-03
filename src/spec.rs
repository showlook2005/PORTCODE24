use crate::schedule::Schedule;
use chrono::{Datelike, DateTime, Duration, LocalResult, TimeZone, Timelike};
use chrono_tz::Tz;

pub const STAR_BIT: u64 = 1 << 63;

pub struct Bounds {
    pub min: u32,
    pub max: u32,
}

pub const SECONDS_BOUNDS: Bounds = Bounds { min: 0, max: 59 };
pub const MINUTES_BOUNDS: Bounds = Bounds { min: 0, max: 59 };
pub const HOURS_BOUNDS: Bounds = Bounds { min: 0, max: 23 };
pub const DOM_BOUNDS: Bounds = Bounds { min: 1, max: 31 };
pub const MONTHS_BOUNDS: Bounds = Bounds { min: 1, max: 12 };
pub const DOW_BOUNDS: Bounds = Bounds { min: 0, max: 6 };

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSchedule {
    pub second: u64,
    pub minute: u64,
    pub hour: u64,
    pub dom: u64,
    pub month: u64,
    pub dow: u64,
    pub location: Option<Tz>,
}

impl SpecSchedule {
    pub fn day_matches(&self, t: &DateTime<Tz>) -> bool {
        let dom_match = (1u64 << t.day()) & self.dom > 0;
        let dow_match = (1u64 << t.weekday().num_days_from_sunday()) & self.dow > 0;
        if self.dom & STAR_BIT > 0 || self.dow & STAR_BIT > 0 {
            dom_match && dow_match
        } else {
            dom_match || dow_match
        }
    }
}

pub fn make_date(loc: Tz, year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> Option<DateTime<Tz>> {
    match loc.with_ymd_and_hms(year, month, day, hour, min, sec) {
        LocalResult::Single(dt) => Some(dt),
        LocalResult::Ambiguous(dt1, _dt2) => Some(dt1),
        LocalResult::None => {
            for h in (hour + 1)..24 {
                if let LocalResult::Single(dt) = loc.with_ymd_and_hms(year, month, day, h, min, sec) {
                    return Some(dt);
                } else if let LocalResult::Ambiguous(dt1, _) = loc.with_ymd_and_hms(year, month, day, h, min, sec) {
                    return Some(dt1);
                }
            }
            None
        }
    }
}

fn add_months(dt: DateTime<Tz>, months: u32, loc: Tz) -> DateTime<Tz> {
    let mut year = dt.year();
    let mut month = dt.month() + months;
    while month > 12 {
        month -= 12;
        year += 1;
    }
    let day = dt.day().min(28);
    make_date(loc, year, month, day, dt.hour(), dt.minute(), dt.second())
        .unwrap_or_else(|| dt + Duration::days(30))
}

fn add_days(dt: DateTime<Tz>, days: i64) -> DateTime<Tz> {
    dt + Duration::days(days)
}

impl Schedule for SpecSchedule {
    fn next(&self, after: DateTime<Tz>) -> Option<DateTime<Tz>> {
        let orig_location = after.timezone();
        let loc = self.location.unwrap_or(orig_location);
        let mut t = after.with_timezone(&loc);

        // Start at the earliest possible time (the upcoming second).
        let nsec = t.timestamp_subsec_nanos();
        t = t + Duration::seconds(1) - Duration::nanoseconds(nsec as i64);

        let mut added = false;
        let year_limit = t.year() + 5;

        'wrap: loop {
            if t.year() > year_limit {
                return None;
            }

            // Month
            while (1u64 << t.month()) & self.month == 0 {
                if !added {
                    added = true;
                    if let Some(d) = make_date(loc, t.year(), t.month(), 1, 0, 0, 0) {
                        t = d;
                    }
                }
                t = add_months(t, 1, loc);
                if let Some(d) = make_date(loc, t.year(), t.month(), 1, 0, 0, 0) {
                    t = d;
                }
                if t.month() == 1 {
                    continue 'wrap;
                }
            }

            // Day
            while !self.day_matches(&t) {
                if !added {
                    added = true;
                    if let Some(d) = make_date(loc, t.year(), t.month(), t.day(), 0, 0, 0) {
                        t = d;
                    }
                }
                t = add_days(t, 1);
                if t.hour() != 0 {
                    if t.hour() > 12 {
                        t = t + Duration::hours((24 - t.hour()) as i64);
                    } else {
                        t = t - Duration::hours(t.hour() as i64);
                    }
                }
                if t.day() == 1 {
                    continue 'wrap;
                }
            }

            // Hour
            while (1u64 << t.hour()) & self.hour == 0 {
                if !added {
                    added = true;
                    if let Some(d) = make_date(loc, t.year(), t.month(), t.day(), t.hour(), 0, 0) {
                        t = d;
                    }
                }
                t = t + Duration::hours(1);
                if t.hour() == 0 {
                    continue 'wrap;
                }
            }

            // Minute
            while (1u64 << t.minute()) & self.minute == 0 {
                if !added {
                    added = true;
                    let sec_nano = t.second() as i64 * 1_000_000_000 + t.timestamp_subsec_nanos() as i64;
                    t = t - Duration::nanoseconds(sec_nano);
                }
                t = t + Duration::minutes(1);
                if t.minute() == 0 {
                    continue 'wrap;
                }
            }

            // Second
            while (1u64 << t.second()) & self.second == 0 {
                if !added {
                    added = true;
                    let nano = t.timestamp_subsec_nanos() as i64;
                    t = t - Duration::nanoseconds(nano);
                }
                t = t + Duration::seconds(1);
                if t.second() == 0 {
                    continue 'wrap;
                }
            }

            return Some(t.with_timezone(&orig_location));
        }
    }
}
