use std::convert::TryFrom;

use askama::{Error, Result};
use chrono::{NaiveDateTime, Utc};
use serde::Serialize;

use crate::server::DisplayString;
use crate::widget::timeattack::QueueEntryAnnotation;

pub(super) fn length<T>(s: &[T]) -> Result<usize> {
    Ok(s.len())
}

pub(super) fn signed(u: &usize) -> Result<i64> {
    match i64::try_from(*u) {
        Ok(s) => Ok(s),
        Err(_e) => Err(Error::Fmt(std::fmt::Error)),
    }
}

pub(super) fn neg(u: &usize) -> Result<i64> {
    match i64::try_from(*u) {
        Ok(s) => Ok(-s),
        Err(_e) => Err(Error::Fmt(std::fmt::Error)),
    }
}

/// For when you don't want to use `serde_json::to_string_pretty`.
pub(super) fn json_ugly<T>(s: &T) -> Result<String>
where
    T: Serialize,
{
    match serde_json::to_string(s) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::from(e)),
    }
}

/// Remove `$o` and `$w` formatting.
pub(super) fn narrow(s: &DisplayString) -> Result<String> {
    Ok(s.formatted.replace("$o", "").replace("$w", ""))
}

/// Format a past date as "New" or "x ago".
pub(super) fn age(x: &NaiveDateTime) -> Result<String> {
    let now = Utc::now().naive_utc();
    assert!(now > *x, "tried to format future date");

    let duration_since = now - *x;
    let days_since = duration_since.num_days();
    let weeks_since = duration_since.num_weeks();
    let months_since = duration_since.num_days() / 30;

    if days_since < 2 {
        return Ok("New".to_string());
    }
    if weeks_since < 2 {
        return Ok(format!("{} days ago", days_since)); // "2..13 days ago"
    }
    if months_since < 2 {
        return Ok(format!("{} weeks ago", weeks_since)); // "2..8 weeks ago"
    }
    if months_since >= 12 {
        return Ok("Long ago".to_string());
    }
    Ok(format!("{} months ago", months_since)) // "2..11 months ago"
}

/// Format an optional, past date as "Never", "Today", "Yesterday", or "x ago".
pub(super) fn when(x: &Option<NaiveDateTime>) -> Result<String> {
    let x = match x {
        Some(x) => x,
        None => return Ok("Never".to_string()),
    };
    let now = Utc::now().naive_utc();
    assert!(now > *x, "tried to format future date");

    let duration_since = now - *x;
    let days_since = duration_since.num_days();
    let weeks_since = duration_since.num_weeks();
    let months_since = duration_since.num_days() / 30;

    if days_since == 0 {
        return Ok("Today".to_string());
    }
    if days_since == 1 {
        return Ok("Yesterday".to_string());
    }
    if weeks_since < 2 {
        return Ok(format!("{} days ago", days_since)); // "2..13 days ago"
    }
    if months_since < 2 {
        return Ok(format!("{} weeks ago", weeks_since)); // "2..8 weeks ago"
    }
    if months_since >= 12 {
        return Ok("Long ago".to_string());
    }
    Ok(format!("{} months ago", months_since)) // "2..11 months ago"
}

// TODO might just wanna use `impl Display` for this
#[allow(dead_code)]
pub(super) fn queue_annotation(p: &QueueEntryAnnotation) -> Result<&str> {
    use QueueEntryAnnotation::*;
    Ok(match p {
        None => "",
        Restart => "Restart",
        Forced => "Forced",
        PlayingNow => "Playing Now",
    })
}
