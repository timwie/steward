use chrono::{NaiveDateTime, Utc};
use serde::Serializer;

use gbx::GameString;

use crate::widget::QueueEntryAnnotation;

/// Remove formatting to make a text more narrow.
pub(super) fn format_narrow<S>(p: &GameString, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&p.formatted.replace("$o", "").replace("$w", ""))
}

pub(super) fn format_map_age<S>(x: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let now = Utc::now().naive_utc();
    let seconds_since = now.timestamp() - x.timestamp();
    assert!(seconds_since >= 0, "tried to format future date");

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

pub(super) fn format_record_age<S>(x: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    format_map_age(x, s)
}

pub(super) fn format_last_played<S>(x: &Option<NaiveDateTime>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let x = match x {
        Some(x) => x,
        None => return s.serialize_str("Never"),
    };
    let now = Utc::now().naive_utc();
    let seconds_since = now.timestamp() - x.timestamp();
    assert!(seconds_since >= 0, "tried to format future date");

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

pub(super) fn format_queue_annotation<S>(p: &QueueEntryAnnotation, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use QueueEntryAnnotation::*;
    let str = match p {
        None => "".to_string(),
        Restart => "Restart".to_string(),
        Forced => "Forced".to_string(),
        PlayingNow => "Playing Now".to_string(),
    };
    s.serialize_str(&str)
}
