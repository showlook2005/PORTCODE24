use chrono::DateTime;
use chrono_tz::Tz;

/// Schedule describes a job's duty cycle.
pub trait Schedule: std::fmt::Debug + Send + Sync {
    /// Returns the next activation time, later than the given time.
    fn next(&self, after: DateTime<Tz>) -> Option<DateTime<Tz>>;
}
