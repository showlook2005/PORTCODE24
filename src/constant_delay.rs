use crate::schedule::Schedule;
use chrono::{DateTime, Duration};
use chrono_tz::Tz;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantDelaySchedule {
    pub delay: Duration,
}

pub fn every(duration: Duration) -> ConstantDelaySchedule {
    let mut dur = duration;
    if dur < Duration::seconds(1) {
        dur = Duration::seconds(1);
    }
    let nanos = dur.num_nanoseconds().unwrap_or(0);
    let rem_nanos = nanos % 1_000_000_000;
    ConstantDelaySchedule {
        delay: dur - Duration::nanoseconds(rem_nanos),
    }
}

impl Schedule for ConstantDelaySchedule {
    fn next(&self, after: DateTime<Tz>) -> Option<DateTime<Tz>> {
        let nanos = after.timestamp_subsec_nanos();
        Some(after + self.delay - Duration::nanoseconds(nanos as i64))
    }
}
