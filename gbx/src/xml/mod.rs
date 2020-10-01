use std::collections::BTreeMap;
use std::convert::TryFrom;

pub(in crate) use from_string::*;
pub(in crate) use from_value::*;
pub(in crate) use to_string::*;
pub(in crate) use to_value::*;

mod from_string;
mod from_value;
mod to_string;
mod to_value;

/// An XML-RPC method call (`<methodCall>`).
#[derive(Clone, Debug, PartialEq)]
pub(in crate) struct Call {
    pub name: String,
    pub args: Vec<Value>,
}

/// An XML-RPC method response (`<methodResponse>`).
pub(in crate) type Response = Result<Value, Fault>;

/// An XML-RPC fault (`<fault>`) of a failed method call.
///
/// Specific errors should be matched by its error message,
/// since the game often uses the code `-1000` for a lot of different errors.
/// When the message is an empty string, the cause has to be
/// deduced from the call, and the context in which it was made.
#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub code: i32,
    pub msg: String,
}

/// An XML-RPC value.
#[derive(Clone, Debug, PartialEq)]
pub(in crate) enum Value {
    /// A 32-bit signed integer (`<i4>`).
    Int(i32),

    /// A boolean value (`<boolean>`, 0 == `false`, 1 == `true`).
    Bool(bool),

    /// A string (`<string>`).
    String(String),

    /// A double-precision IEEE 754 floating point number (`<double>`).
    Double(f64),

    /// Base64-encoded binary data (`<base64>`).
    Base64(Vec<u8>),

    /// A mapping of named values (`<struct>`).
    Struct(BTreeMap<String, Value>),

    /// A list of arbitrary (heterogeneous) values (`<array>`).
    Array(Vec<Value>),
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Int(i32::try_from(v).expect("cannot fit u32 into Value::Int"))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Double(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Base64(v)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(v: BTreeMap<String, Value>) -> Self {
        Value::Struct(v)
    }
}

impl<T> From<Vec<T>> for Value
where
    Value: From<T>,
{
    fn from(vs: Vec<T>) -> Self {
        Value::Array(vs.into_iter().map(|v| v.into()).collect())
    }
}

#[cfg(test)]
mod tests {
    use from_string::base64_decode;

    use super::*;

    #[test]
    fn base64_decode_encode_roundtrip() {
        // Test that we can take a base 64 encoded validation replay
        // that was returned by the dedicated server,
        // decode it, and re-encode it to get the original input string.

        let b64_orig = include_str!("validation_replay_base64");
        let bytes = base64_decode(b64_orig);
        let b64 = base64_encode(&bytes);
        assert_eq!(b64_orig, b64);
    }
}
