use std::time::Duration;

/// All dates and times are UTC.
pub type DateTime = chrono::DateTime<chrono::Utc>;

/// Returns the current date and time in UTC
pub fn now() -> DateTime {
    chrono::offset::Utc::now()
}

/// Returns a date and time in UTC from a timestamp in seconds
pub fn from_timestamp(timestamp: i64) -> DateTime {
    DateTime::from_timestamp(timestamp, 0).expect("could not get date from seconds")
}

/// Returns a date and time in UTC from a timestamp in milliseconds
pub fn from_timestamp_millis(timestamp: i64) -> DateTime {
    DateTime::from_timestamp_millis(timestamp).expect("could not get date from milliseconds")
}

/// Returns a date and time after given duration
pub fn date_after(duration: Duration) -> DateTime {
    now() + duration
}

/// Returns current date and time with given offset in seconds
pub fn after_seconds(seconds: u64) -> DateTime {
    date_after(Duration::from_secs(seconds))
}
