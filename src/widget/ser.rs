use std::time::SystemTime;

use serde::Serializer;

use crate::controller::QueuePriority;

/// Remove formatting to make a text more narrow.
pub fn format_narrow<S>(p: &str, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&p.replace("$o", "").replace("$w", ""))
}

pub fn format_map_age<S>(x: &SystemTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let seconds_since = match x.elapsed() {
        Ok(duration) => duration.as_secs(),
        Err(_) => return s.serialize_str(""),
    };
    let days_since = seconds_since / 60 / 60 / 24; // div rounds down
    let weeks_since = days_since / 7;
    let months_since = days_since / 30;

    if days_since < 2 {
        return s.serialize_str("New");
    }
    if weeks_since < 2 {
        return s.serialize_str(&format!("{} days ago", days_since)); // "2..13 days ago"
    }
    if months_since < 2 {
        return s.serialize_str(&format!("{} weeks ago", weeks_since)); // "2..8 weeks ago"
    }
    if months_since >= 12 {
        return s.serialize_str("Long ago");
    }
    s.serialize_str(&format!("{} months ago", months_since)) // "2..11 months ago"
}

pub fn format_record_age<S>(x: &SystemTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    format_map_age(x, s)
}

pub fn format_last_played<S>(x: &Option<SystemTime>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let x = match x {
        Some(x) => x,
        None => return s.serialize_str("Never"),
    };
    let seconds_since = match x.elapsed() {
        Ok(duration) => duration.as_secs(),
        Err(_) => return s.serialize_str(""),
    };
    let days_since = seconds_since / 60 / 60 / 24; // div rounds down
    let weeks_since = days_since / 7;
    let months_since = days_since / 30;

    if days_since == 0 {
        return s.serialize_str("Today");
    }
    if days_since == 1 {
        return s.serialize_str("Yesterday");
    }
    if weeks_since < 2 {
        return s.serialize_str(&format!("{} days ago", days_since)); // "2..13 days ago"
    }
    if months_since < 2 {
        return s.serialize_str(&format!("{} weeks ago", weeks_since)); // "2..8 weeks ago"
    }
    if months_since >= 12 {
        return s.serialize_str("Long ago");
    }
    s.serialize_str(&format!("{} months ago", months_since)) // "2..11 months ago"
}

pub fn format_queue_priority<S>(p: &QueuePriority, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use QueuePriority::*;
    let str = match p {
        NoRestart => "Playing Now".to_string(),
        VoteRestart => "Restart".to_string(),
        Force(_) => "Force".to_string(),
        Score(score) if *score >= 0 => format!("+{}", *score),
        Score(score) => score.to_string(),
    };
    s.serialize_str(&str)
}
