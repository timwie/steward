use std::collections::BTreeMap;

use quick_xml::{events::Event, Reader};

use anyhow::{anyhow, Context, Result};

use crate::xml::{Call, Fault, Response, Value};

/// Try to parse a `<methodCall>` in the input string.
pub fn read_method_call(input: &str) -> Result<Call> {
    let mut reader = Reader::from_str(input);
    reader.expand_empty_elements(true);
    reader.trim_text(true);

    let mut buf = Vec::new();
    expect_decl(&mut reader, &mut buf)?;

    expect_tag(b"methodCall", &mut reader, &mut buf)?;
    expect_tag(b"methodName", &mut reader, &mut buf)?;
    let method_name = reader.read_text(b"methodName", &mut buf)?;

    let mut result = Call {
        name: method_name,
        args: Vec::new(),
    };

    expect_tag(b"params", &mut reader, &mut buf)?;
    let mut vals = read_params(&mut reader, &mut buf)?;
    result.args.append(&mut vals);

    reader.read_to_end(b"methodCall", &mut buf)?;

    Ok(result)
}

/// Try to parse a `<methodResponse>` in the input string.
pub fn read_method_response(input: &str) -> Result<Response> {
    let mut reader = Reader::from_str(input);
    reader.expand_empty_elements(true);
    reader.trim_text(true);

    let mut buf = Vec::new();
    expect_decl(&mut reader, &mut buf)?;

    expect_tag(b"methodResponse", &mut reader, &mut buf)?;

    match reader.read_event(&mut buf)? {
        Event::Start(ref e) if e.name() == b"params" => {
            let vals = read_params(&mut reader, &mut buf)?;
            reader.read_to_end(b"methodResponse", &mut buf)?;
            match vals.into_iter().next() {
                Some(first_val) => Ok(Ok(first_val)),
                None => Err(anyhow!("expected single param for methodResponse")),
            }
        }
        Event::Start(ref e) if e.name() == b"fault" => {
            expect_tag(b"value", &mut reader, &mut buf)?;
            match read_value(&mut reader, &mut buf)? {
                Value::Struct(members) => {
                    let code = match members.get("faultCode") {
                        Some(Value::Int(code)) => code,
                        _ => return Err(anyhow!("Cannot read fault: {:?}", members)),
                    };
                    let msg = match members.get("faultString") {
                        Some(Value::String(msg)) => msg,
                        _ => return Err(anyhow!("Cannot read fault: {:?}", members)),
                    };
                    Ok(Err(Fault {
                        code: *code,
                        msg: msg.clone(),
                    }))
                }
                v => Err(anyhow!("Cannot read fault: {:?}", v)),
            }
        }
        ev => tag_err(ev, "<methodResponse> or <fault>"),
    }
}

fn read_params<B>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<Vec<Value>>
where
    B: std::io::BufRead,
{
    let mut vals = Vec::new();
    loop {
        match reader.read_event(buf)? {
            Event::Start(e) if e.name() == b"param" => {
                let val = read_param(reader, buf)?;
                vals.push(val);
            }
            Event::End(e) if e.name() == b"params" => break,
            ev => {
                return tag_err(ev, "<param> or </params>");
            }
        };
    }
    Ok(vals)
}

fn read_param<B>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<Value>
where
    B: std::io::BufRead,
{
    expect_tag(b"value", reader, buf)?;
    let val = read_value(reader, buf)?;
    reader.read_to_end(b"param", buf)?;
    Ok(val)
}

fn read_value<B>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<Value>
where
    B: std::io::BufRead,
{
    let res: Result<Value> = match reader.read_event(buf)? {
        Event::Start(e) if e.name() == b"i4" => {
            let i: i32 = reader
                .read_text(b"i4", buf)?
                .parse()
                .context("Expected a valid <i4> value")?;
            Ok(Value::Int(i))
        }
        Event::Start(e) if e.name() == b"int" => {
            let i: i32 = reader
                .read_text(b"int", buf)?
                .parse()
                .context("Expected a valid <i4> value")?;
            Ok(Value::Int(i))
        }
        Event::Start(e) if e.name() == b"double" => {
            let f: f64 = reader
                .read_text(b"double", buf)?
                .parse()
                .context("Expected a valid <double> value")?;
            Ok(Value::Double(f))
        }
        Event::Start(e) if e.name() == b"boolean" => {
            match reader.read_text(b"boolean", buf)?.as_ref() {
                "0" => Ok(Value::Bool(false)),
                "1" => Ok(Value::Bool(true)),
                txt => Err(anyhow!("Expected 0 or 1, got {}", txt)),
            }
        }
        Event::Start(e) if e.name() == b"string" => {
            let str = reader.read_text(b"string", buf)?;
            Ok(Value::String(str))
        }
        Event::Start(e) if e.name() == b"base64" => {
            let str = reader.read_text(b"base64", buf)?;
            Ok(Value::Base64(base64_decode(&str)))
        }
        Event::Start(e) if e.name() == b"array" => {
            let arr = read_array(reader, buf)?;
            Ok(arr)
        }
        Event::Start(e) if e.name() == b"struct" => {
            let strct = read_struct(reader, buf)?;
            Ok(strct)
        }
        ev => tag_err(
            ev,
            "<i4>, <int>, <double>, <boolean>, <string>, <base64>, <array> or <struct>",
        ),
    };
    reader.read_to_end(b"value", buf)?;
    Ok(res?)
}

/// Decode Base64 to bytes.
///
/// # Panics
/// Panics if decoding fails.
pub fn base64_decode(b64: &str) -> Vec<u8> {
    // base64 crate cannot decode with whitespace, but the server
    // gives us text that is wrapped at 76 characters (specified by MIME) with '\r\n'
    let mut str_no_wrap = String::with_capacity(b64.len());
    for c in b64.chars() {
        // As suggested here: https://github.com/marshallpierce/rust-base64/issues/105#issuecomment-497858566
        if b" \n\t\r\x0b\x0c".contains(&(c as u8)) {
            continue;
        }
        str_no_wrap.push(c);
    }
    base64::decode(&str_no_wrap).expect("Expected a valid <base64> value")
}

fn read_array<B>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<Value>
where
    B: std::io::BufRead,
{
    expect_tag(b"data", reader, buf)?;

    let mut vals = Vec::new();
    loop {
        match reader.read_event(buf)? {
            Event::Start(e) if e.name() == b"value" => {
                let val = read_value(reader, buf)?;
                vals.push(val);
            }
            Event::End(e) if e.name() == b"data" => {
                reader.read_to_end(b"array", buf)?;
                break;
            }
            ev => {
                return tag_err(ev, "<value> or </data>");
            }
        };
    }
    Ok(Value::Array(vals))
}

fn read_struct<B>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<Value>
where
    B: std::io::BufRead,
{
    let mut members = BTreeMap::new();
    loop {
        match reader.read_event(buf)? {
            Event::Start(e) if e.name() == b"member" => {
                expect_tag(b"name", reader, buf)?;
                let mem_name = reader.read_text(b"name", buf)?;
                expect_tag(b"value", reader, buf)?;
                let mem_val = read_value(reader, buf)?;
                reader.read_to_end(b"member", buf)?;
                members.insert(mem_name, mem_val);
            }
            Event::End(e) if e.name() == b"struct" => break,
            ev => {
                return tag_err(ev, "<member> or </struct>");
            }
        };
    }
    Ok(Value::Struct(members))
}

fn expect_decl<B>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<()>
where
    B: std::io::BufRead,
{
    match reader.read_event(buf)? {
        Event::Decl(_) => Ok(()),
        ev => tag_err(ev, "<xml>"),
    }
}

fn expect_tag<B>(end: &[u8], reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<()>
where
    B: std::io::BufRead,
{
    match reader.read_event(buf)? {
        Event::Start(ref e) if e.name() == end => Ok(()),
        ev => tag_err(ev, std::str::from_utf8(end)?),
    }
}

fn tag_err<T>(got: Event, expected: &str) -> Result<T> {
    Err(anyhow!(
        "XML parser got {:?}, but expected {}",
        got,
        expected
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_callback_no_params() {
        let expected = Call {
            name: "TrackMania.PlayerConnect".to_string(),
            args: vec![],
        };
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <methodCall>
               <methodName>TrackMania.PlayerConnect</methodName>
               <params>
               </params>
            </methodCall>
        "#;
        assert_eq!(expected, read_method_call(xml).unwrap())
    }

    #[test]
    fn parse_callback_single_param() {
        let expected = Call {
            name: "TrackMania.PlayerConnect".to_string(),
            args: vec![Value::String("tim".to_string())],
        };
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <methodCall>
               <methodName>TrackMania.PlayerConnect</methodName>
               <params>
                 <param>
                     <value>
                        <string>tim</string>
                     </value>
                  </param>
               </params>
            </methodCall>
        "#;
        assert_eq!(expected, read_method_call(xml).unwrap())
    }

    #[test]
    fn parse_callback_multi_params() {
        let expected = Call {
            name: "TrackMania.PlayerConnect".to_string(),
            args: vec![Value::String("tim".to_string()), Value::Bool(false)],
        };
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
            <methodCall>
               <methodName>TrackMania.PlayerConnect</methodName>
               <params>
                 <param>
                     <value>
                        <string>tim</string>
                     </value>
                  </param>
                  <param>
                     <value>
                        <boolean>0</boolean>
                     </value>
                  </param>
               </params>
            </methodCall>
        "#;
        assert_eq!(expected, read_method_call(xml).unwrap())
    }

    #[test]
    fn parse_callback_empty_array() {
        let expected = Call {
            name: "TrackMania.PlayerConnect".to_string(),
            args: vec![Value::Array(vec![])],
        };
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <methodCall>
               <methodName>TrackMania.PlayerConnect</methodName>
               <params><param><value><array><data>
               </data></array></value></param></params>
            </methodCall>
        "#;
        assert_eq!(expected, read_method_call(xml).unwrap())
    }

    #[test]
    fn parse_callback_array() {
        let expected = Call {
            name: "TrackMania.PlayerConnect".to_string(),
            args: vec![Value::Array(vec![Value::Int(42), Value::Double(3.14)])],
        };
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <methodCall>
               <methodName>TrackMania.PlayerConnect</methodName>
               <params><param><value><array><data>
                 <value>
                    <i4>42</i4>
                 </value>
                 <value>
                    <double>3.14</double>
                 </value>
               </data></array></value></param></params>
            </methodCall>
        "#;
        assert_eq!(expected, read_method_call(xml).unwrap())
    }

    #[test]
    fn parse_callback_struct() {
        let mut expected_members = BTreeMap::new();
        expected_members.insert("Login".to_string(), Value::String("tim".to_string()));
        let expected = Call {
            name: "TrackMania.PlayerInfoChanged".to_string(),
            args: vec![Value::Struct(expected_members)],
        };
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <methodCall>
           <methodName>TrackMania.PlayerInfoChanged</methodName>
           <params>
              <param>
                 <value>
                    <struct>
                       <member>
                          <name>Login</name>
                          <value>
                             <string>tim</string>
                          </value>
                       </member>
                    </struct>
                 </value>
              </param>
           </params>
        </methodCall>
        "#;
        assert_eq!(expected, read_method_call(xml).unwrap())
    }

    #[test]
    #[ignore]
    fn parse_manialink_fault() {
        // This fault is a bit tricky, since it includes a <string> tag inside
        // another <string> tag. Fixing it would give us a better error message
        // for a bad Manialink.
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
            <methodResponse>
                <fault>
                    <value>
                        <struct>
                            <member>
                                <name>faultCode</name>
                                <value><int>-503</int></value>
                            </member>
                            <member>
                                <name>faultString</name>
                                <value><string>
                                Call XML not a proper XML-RPC call. Expected <string> to have 0 children, found 1
                                </string></value>
                            </member>
                        </struct>
                    </value>
                </fault>
            </methodResponse>
        "#;
        assert!(read_method_response(xml).is_ok());
    }
}
