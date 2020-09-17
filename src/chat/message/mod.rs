use std::fmt::Formatter;
pub use to_player::*;
pub use to_server::*;

mod to_player;
mod to_server;

/// Turn the number of milliseconds into a readable run time,
/// f.e. '00:48:051' for '48051'.
pub(self) fn fmt_time(millis: usize) -> String {
    let secs = millis / 1000;
    let millis = millis % 1000;
    let mins = secs / 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:03}", mins, secs, millis)
}

/// Either `"no <word>"`, `"one <word>"` or `"<amount> <word>s"`.
pub(self) fn pluralize(word: &str, amount: usize) -> String {
    let prefix = match amount {
        0 => "no".to_string(),
        1 => "one".to_string(),
        n => n.to_string(),
    };
    let suffix = if amount == 1 { "" } else { "s" };
    format!("{} {}{}", prefix, word, suffix)
}

pub(self) fn write_start_message(f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}{}🔊 ", RESET, NOTICE)
}

pub(self) fn write_and_reset<T>(f: &mut Formatter<'_>, text: T) -> std::fmt::Result
where
    T: std::fmt::Display,
{
    write!(f, "{}{}{}{}", RESET, text, RESET, NOTICE)
}

pub(self) fn write_highlighted<T>(f: &mut Formatter<'_>, text: T) -> std::fmt::Result
where
    T: std::fmt::Display,
{
    write!(f, "{}{}{}{}{}", RESET, HIGHLIGHT, text, RESET, NOTICE)
}

const HIGHLIGHT: &str = "$fff$o";

const NOTICE: &str = "$fc0";

const RESET: &str = "$z$fff$s";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fmt_time() {
        assert_eq!("00:21:105", fmt_time(21105))
    }
}
