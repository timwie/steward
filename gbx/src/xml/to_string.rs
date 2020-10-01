use quick_xml::events::{BytesEnd, BytesStart, BytesText};
use quick_xml::{events::Event, Writer};

use crate::xml::{Call, Value};

/// Try to compose a `<methodCall>`.
///
/// # Panics
/// Panics if the composition fails.
pub(in crate) fn write_method_call(call: &Call) -> Vec<u8> {
    try_write_method_call(call)
        .unwrap_or_else(|err| panic!("failed to compose method call from {:?}: {}", call, err))
}

fn try_write_method_call(call: &Call) -> Result<Vec<u8>, quick_xml::Error> {
    let mut writer = Writer::new(Vec::new());

    writer.write(br#"<?xml version="1.0" encoding="utf-8"?>"#)?;

    write_start_tag(b"methodCall", &mut writer)?;
    write_tag(b"methodName", &call.name, &mut writer)?;

    write_start_tag(b"params", &mut writer)?;
    for value in &call.args {
        write_start_tag(b"param", &mut writer)?;
        write_value(value, &mut writer)?;
        write_end_tag(b"param", &mut writer)?;
    }
    write_end_tag(b"params", &mut writer)?;
    write_end_tag(b"methodCall", &mut writer)?;

    Ok(writer.into_inner())
}

fn write_tag<W>(tag: &[u8], text: &str, writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    write_start_tag(tag, writer)?;
    write_text(text, writer)?;
    write_end_tag(tag, writer)?;
    Ok(())
}

fn write_safe_tag<W>(tag: &[u8], text: &str, writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    write_start_tag(tag, writer)?;
    write_safe_text(text, writer)?;
    write_end_tag(tag, writer)?;
    Ok(())
}

fn write_text<W>(text: &str, writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    writer.write_event(Event::Text(BytesText::from_plain_str(text)))?;
    Ok(())
}

fn write_safe_text<W>(text: &str, writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    writer.write_event(Event::Text(BytesText::from_escaped_str(text)))?;
    Ok(())
}

fn write_start_tag<W>(tag: &[u8], writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    writer.write_event(Event::Start(BytesStart::borrowed_name(tag)))?;
    Ok(())
}

fn write_end_tag<W>(tag: &[u8], writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    writer.write_event(Event::End(BytesEnd::borrowed(tag)))?;
    Ok(())
}

fn write_value<W>(value: &Value, writer: &mut Writer<W>) -> Result<(), quick_xml::Error>
where
    W: std::io::Write,
{
    write_start_tag(b"value", writer)?;
    match value {
        Value::Int(i) => {
            write_tag(b"i4", &i.to_string(), writer)?;
        }
        Value::Double(f) => {
            write_tag(b"double", &f.to_string(), writer)?;
        }
        Value::Bool(b) => {
            write_tag(b"boolean", if *b { "1" } else { "0" }, writer)?;
        }
        Value::String(s) => {
            write_safe_tag(b"string", &s, writer)?;
        }
        Value::Base64(b) => {
            write_tag(b"string", &base64_encode(b), writer)?;
        }
        Value::Array(vs) => {
            write_start_tag(b"array", writer)?;
            write_start_tag(b"data", writer)?;
            for v in vs {
                write_value(v, writer)?;
            }
            write_end_tag(b"data", writer)?;
            write_end_tag(b"array", writer)?;
        }
        Value::Struct(ms) => {
            write_start_tag(b"struct", writer)?;
            for (name, v) in ms {
                write_start_tag(b"member", writer)?;
                write_safe_tag(b"name", &name, writer)?;
                write_value(v, writer)?;
                write_end_tag(b"member", writer)?;
            }
            write_end_tag(b"struct", writer)?;
        }
    }
    write_end_tag(b"value", writer)?;
    Ok(())
}

/// Encode bytes to Base64.
///
/// Lines will be wrapped at a maximum line length of 76 characters
/// (specified by MIME) with '\r\n' to mimic the dedicated server's behavior.
pub fn base64_encode(bytes: &[u8]) -> String {
    const LINE_LENGTH: usize = 76;

    let str_no_wrap = base64::encode(bytes);

    let nb_chars_needed = str_no_wrap.len() + str_no_wrap.len() / LINE_LENGTH * 2;

    let mut str = String::with_capacity(nb_chars_needed);

    for (i, c) in str_no_wrap.chars().enumerate() {
        if i > 0 && i % LINE_LENGTH == 0 {
            str.push('\r');
            str.push('\n');
        }
        str.push(c);
    }

    str
}
