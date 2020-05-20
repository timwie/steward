// Thanks to https://github.com/belak/serde-xmlrpc/blob/master/src/value/ser.rs
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
use std::convert::TryFrom;
use std::fmt::Formatter;

use serde::Serialize;

use crate::xml::Value;

/// Compose a `Value` from a `T`.
/// - `Value::Struct` can be built from struct instances
/// - `Value::Array` are built from vectors
/// - the remaining `Value` variants are built from primitive types
///
/// # Panics
/// Panics if the composition fails.
pub fn to_value<T>(t: T) -> Value
where
    T: serde::ser::Serialize,
    T: std::fmt::Debug,
{
    t.serialize(ValueSerializer)
        .unwrap_or_else(|err| panic!("failed to serialize {:?} as value: {}", t, err))
}

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error(msg.to_string())
    }
}

struct ValueSerializer;

impl serde::Serializer for ValueSerializer {
    type Error = Error;
    type Ok = Value;

    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = SerializeVec;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeMap;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Error> {
        Ok(Value::Int(i32::from(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Error> {
        Ok(Value::Int(i32::from(v)))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Error> {
        Ok(Value::Int(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Error> {
        Ok(Value::Int(
            i32::try_from(v).expect("cannot fit i64 into Value::Int"),
        ))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Error> {
        Ok(Value::Int(i32::from(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Error> {
        Ok(Value::Int(i32::from(v)))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Error> {
        Ok(Value::Int(
            i32::try_from(v).expect("cannot fit u32 into Value::Int"),
        ))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Error> {
        Ok(Value::Int(
            i32::try_from(v).expect("cannot fit u64 into Value::Int"),
        ))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Error> {
        Ok(Value::Double(f64::from(v)))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Error> {
        Ok(Value::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Error> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Error> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Error> {
        Ok(Value::Base64(v.into()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Error> {
        unimplemented!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Error>
    where
        T: Serialize,
    {
        value.serialize(ValueSerializer)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Error> {
        unimplemented!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Error> {
        Ok(Value::Struct(BTreeMap::new()))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Error> {
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Error>
    where
        T: Serialize,
    {
        value.serialize(ValueSerializer)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Error>
    where
        T: Serialize,
    {
        unimplemented!();
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        self.serialize_tuple(len.unwrap_or(0))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        self.serialize_tuple(len)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Ok(SerializeMap {
            map: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        self.serialize_map(Some(len))
    }
}

#[doc(hidden)]
pub struct SerializeVec {
    vec: Vec<Value>,
}

impl serde::ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        self.vec.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Error> {
        Ok(Value::Array(self.vec))
    }
}

impl serde::ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleVariant for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[doc(hidden)]
pub struct SerializeMap {
    map: BTreeMap<String, Value>,
    next_key: Option<String>,
}

impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        // We can only serialize keys if they can be converted to strings
        match key.serialize(ValueSerializer)? {
            Value::Int(v) => {
                self.next_key = Some(v.to_string());
                Ok(())
            }
            Value::Bool(v) => {
                self.next_key = Some(v.to_string());
                Ok(())
            }
            Value::String(s) => {
                self.next_key = Some(s);
                Ok(())
            }
            Value::Double(v) => {
                self.next_key = Some(v.to_string());
                Ok(())
            }
            _ => Err(Error(
                "Key must be an int, int64, bool, string, char, or float.".to_string(),
            )),
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let key = self
            .next_key
            .take()
            .expect("serialize_value called before serialize_key");
        let value = value.serialize(ValueSerializer)?;

        self.map.insert(key, value);

        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Struct(self.map))
    }
}

impl serde::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        serde::ser::SerializeMap::serialize_key(self, key)?;
        serde::ser::SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeMap::end(self)
    }
}

impl serde::ser::SerializeStructVariant for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        serde::ser::SerializeMap::serialize_key(self, key)?;
        serde::ser::SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeMap::end(self)
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    use serde::Serialize;

    use crate::xml::Value;

    use super::ValueSerializer;

    #[derive(Serialize, Debug, PartialEq)]
    struct StringStruct {
        hello: String,
    }

    #[test]
    fn int_to_value() {
        let expected_value = Value::Int(42);
        let i: i32 = 42;
        let value = i.serialize(ValueSerializer).unwrap();
        assert_eq!(expected_value, value);
    }

    #[test]
    fn vec_to_value() {
        let expected_value = Value::Array(vec![Value::String("hello world".to_string())]);
        let vec: Vec<String> = vec!["hello world".to_string()];
        let value = vec.serialize(ValueSerializer).unwrap();
        assert_eq!(expected_value, value);
    }

    #[test]
    fn map_to_value() {
        let expected_value = Value::Struct(BTreeMap::from_iter(
            vec![("hello".to_string(), Value::String("world".to_string()))].into_iter(),
        ));
        let obj = StringStruct {
            hello: "world".to_string(),
        };
        let value = obj.serialize(ValueSerializer).unwrap();
        assert_eq!(expected_value, value);
    }
}
