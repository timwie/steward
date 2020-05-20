// Thanks to https://github.com/belak/serde-xmlrpc/blob/master/src/value/de.rs
//
// MIT License
//
// Copyright (c) 2020 Kaleb Elwert
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::collections::BTreeMap;

use serde::de::Visitor;
use serde::export::Formatter;
use serde::forward_to_deserialize_any;

use crate::xml::Value;

/// Deserialize a `Value` to a `T`.
/// - struct instances can be built from `Value::Struct`
/// - vectors are built from `Value::Array`
/// - primitive types are lifted out of the remaining `Value` variants.
pub fn from_value<T>(value: Value) -> Result<T, FromValueError>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(ValueDeserializer {
        value: value.clone(),
    })
    .map_err(|de_err| FromValueError {
        input: value,
        error_msg: de_err.0,
    })
}

#[derive(Debug)]
pub struct FromValueError {
    pub input: Value,
    pub error_msg: String,
}

impl std::fmt::Display for FromValueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to deserialize {:?} into desired type: {}",
            self.input, self.error_msg
        )
    }
}

#[derive(Debug)]
struct DeError(String);

impl std::fmt::Display for DeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DeError {}

impl serde::de::Error for DeError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        DeError(msg.to_string())
    }
}

struct ValueDeserializer {
    value: Value,
}

impl<'de> serde::Deserializer<'de> for ValueDeserializer {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Int(v) => visitor.visit_i32(v),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::String(v) => visitor.visit_string(v),
            Value::Double(v) => visitor.visit_f64(v),
            Value::Base64(v) => visitor.visit_bytes(v.as_slice()),
            Value::Struct(v) => {
                let map_deserializer = MapDeserializer::new(v);
                visitor.visit_map(map_deserializer)
            }
            Value::Array(v) => {
                let seq_deserializer = SeqDeserializer::new(v);
                visitor.visit_seq(seq_deserializer)
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    forward_to_deserialize_any!(
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    );
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Value>,
}

impl SeqDeserializer {
    fn new(vec: Vec<Value>) -> Self {
        SeqDeserializer {
            iter: vec.into_iter(),
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for SeqDeserializer {
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(ValueDeserializer { value }).map(Some),
            None => Ok(None),
        }
    }
}

struct MapDeserializer {
    iter: <BTreeMap<String, Value> as IntoIterator>::IntoIter,
    value: Option<Value>,
}

impl MapDeserializer {
    fn new(map: BTreeMap<String, Value>) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for MapDeserializer {
    type Error = DeError;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(ValueDeserializer {
                    value: Value::String(key),
                })
                .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, DeError>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(ValueDeserializer { value }),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    use serde::Deserialize;
    use serde_bytes::ByteBuf;

    use crate::xml::Value;

    use super::*;

    #[derive(Deserialize, Debug, PartialEq)]
    struct StringStruct {
        hello: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct ByteStruct {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NumberStruct {
        number: u32,
    }

    #[test]
    fn value_to_simple_struct() {
        let value = Value::Struct(BTreeMap::from_iter(
            vec![("hello".to_string(), Value::String("world".to_string()))].into_iter(),
        ));
        let _ = StringStruct::deserialize(ValueDeserializer { value }).unwrap();
    }

    #[test]
    fn value_to_bytes() {
        let data = Value::Base64(vec![71, 66, 88, 6, 0]);
        let res = from_value::<ByteBuf>(data);
        res.unwrap();
    }

    #[test]
    fn value_to_bytes_in_struct() {
        let value = Value::Struct(BTreeMap::from_iter(
            vec![("data".to_string(), Value::Base64(vec![71, 66, 88, 6, 0]))].into_iter(),
        ));
        let _ = ByteStruct::deserialize(ValueDeserializer { value }).unwrap();
    }

    #[test]
    fn value_cast_to_u32() {
        let value = Value::Struct(BTreeMap::from_iter(
            vec![("number".to_string(), Value::Int(42))].into_iter(),
        ));
        assert_eq!(
            42,
            NumberStruct::deserialize(ValueDeserializer { value })
                .unwrap()
                .number
        );
    }

    #[test]
    fn value_bad_cast_to_u32() {
        let value = Value::Struct(BTreeMap::from_iter(
            vec![("number".to_string(), Value::Int(-42))].into_iter(),
        ));
        assert!(NumberStruct::deserialize(ValueDeserializer { value }).is_err());
    }
}
