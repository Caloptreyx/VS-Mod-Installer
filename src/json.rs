//! A lenient JSON reader for files Vintage Story itself reads with Newtonsoft.Json
//! (`modinfo.json`, `serverconfig.json`). Newtonsoft accepts more than strict JSON, and mod
//! authors rely on it: `//` and `/* */` comments, trailing commas, single-quoted strings and
//! unquoted property names. Property names are matched case-insensitively by [`get`].
use serde_json::{Map, Number, Value};

const MAX_DEPTH: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: &'static str,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}

pub fn parse(text: &str) -> Result<Value, ParseError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut parser = Parser {
        bytes: text.as_bytes(),
        text,
        pos: 0,
    };
    let value = parser.value(0)?;
    parser.skip_ws()?;
    if parser.pos < parser.bytes.len() {
        return Err(parser.error("unexpected content after the value"));
    }
    Ok(value)
}

/// The value of `key` in an object, ignoring the case of property names (the last match wins,
/// like Newtonsoft's populate order).
pub fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_object()?
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| value)
        .next_back()
}

/// A string, number or boolean as text; `None` for null, arrays and objects.
pub fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, message: &'static str) -> ParseError {
        ParseError {
            offset: self.pos,
            message,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0c) => self.pos += 1,
                Some(b'/') => match self.bytes.get(self.pos + 1) {
                    Some(b'/') => {
                        while let Some(byte) = self.peek() {
                            if byte == b'\n' {
                                break;
                            }
                            self.pos += 1;
                        }
                    }
                    Some(b'*') => {
                        let start = self.pos;
                        self.pos += 2;
                        loop {
                            match self.peek() {
                                None => {
                                    self.pos = start;
                                    return Err(self.error("unterminated comment"));
                                }
                                Some(b'*') if self.bytes.get(self.pos + 1) == Some(&b'/') => {
                                    self.pos += 2;
                                    break;
                                }
                                Some(_) => self.pos += 1,
                            }
                        }
                    }
                    _ => return Ok(()),
                },
                // a non-breaking space or other Unicode whitespace
                Some(byte) if byte >= 0x80 => {
                    let ch = self.text[self.pos..].chars().next().unwrap_or('\0');
                    if !ch.is_whitespace() {
                        return Ok(());
                    }
                    self.pos += ch.len_utf8();
                }
                _ => return Ok(()),
            }
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.skip_ws()?;
        match self.peek() {
            None => Err(self.error("unexpected end of input")),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(quote @ (b'"' | b'\'')) => Ok(Value::String(self.string(quote)?)),
            Some(b'-' | b'+' | b'.' | b'0'..=b'9') => self.number(),
            Some(_) => {
                let start = self.pos;
                let word = self.identifier();
                match word {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    "null" | "undefined" => Ok(Value::Null),
                    _ => {
                        self.pos = start;
                        Err(self.error("unexpected character"))
                    }
                }
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.pos += 1;
        let mut map = Map::new();
        loop {
            self.skip_ws()?;
            match self.peek() {
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(map));
                }
                // empty entries such as `{ "a": 1,, }` are tolerated, like trailing commas
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                None => return Err(self.error("unterminated object")),
                _ => {}
            }
            let key = match self.peek() {
                Some(quote @ (b'"' | b'\'')) => self.string(quote)?,
                _ => {
                    let word = self.identifier();
                    if word.is_empty() {
                        return Err(self.error("expected a property name"));
                    }
                    word.to_string()
                }
            };
            self.skip_ws()?;
            if self.peek() != Some(b':') {
                return Err(self.error("expected ':'"));
            }
            self.pos += 1;
            let value = self.value(depth + 1)?;
            map.insert(key, value);
            self.skip_ws()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {}
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws()?;
            match self.peek() {
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                None => return Err(self.error("unterminated array")),
                _ => {}
            }
            items.push(self.value(depth + 1)?);
            self.skip_ws()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {}
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn identifier(&mut self) -> &'a str {
        let text = self.text;
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        &text[start..self.pos]
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let text = self.text;
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = text[start..self.pos].trim_start_matches('+');
        if let Ok(int) = raw.parse::<i64>() {
            return Ok(Value::Number(int.into()));
        }
        if let Ok(int) = raw.parse::<u64>() {
            return Ok(Value::Number(int.into()));
        }
        match raw.parse::<f64>().ok().and_then(Number::from_f64) {
            Some(number) => Ok(Value::Number(number)),
            None => {
                self.pos = start;
                Err(self.error("invalid number"))
            }
        }
    }

    fn string(&mut self, quote: u8) -> Result<String, ParseError> {
        let start = self.pos;
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(byte) = self.peek() else {
                self.pos = start;
                return Err(self.error("unterminated string"));
            };
            if byte == quote {
                self.pos += 1;
                return Ok(out);
            }
            if byte == b'\\' {
                self.pos += 1;
                let Some(escaped) = self.peek() else {
                    return Err(self.error("unterminated escape"));
                };
                self.pos += 1;
                match escaped {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'u' => out.push(self.unicode_escape()?),
                    // a backslash before a line break continues the string
                    b'\n' => {}
                    other if other < 0x80 => out.push(other as char),
                    _ => {
                        self.pos -= 1;
                        let ch = self.text[self.pos..].chars().next().unwrap_or('\u{fffd}');
                        out.push(ch);
                        self.pos += ch.len_utf8();
                    }
                }
                continue;
            }
            let ch = self.text[self.pos..].chars().next().unwrap_or('\u{fffd}');
            out.push(ch);
            self.pos += ch.len_utf8();
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let digits = self
            .text
            .get(self.pos..self.pos + 4)
            .ok_or_else(|| self.error("invalid unicode escape"))?;
        let value =
            u32::from_str_radix(digits, 16).map_err(|_| self.error("invalid unicode escape"))?;
        self.pos += 4;
        Ok(value)
    }

    fn unicode_escape(&mut self) -> Result<char, ParseError> {
        let first = self.hex4()?;
        if (0xd800..0xdc00).contains(&first)
            && self.bytes.get(self.pos) == Some(&b'\\')
            && self.bytes.get(self.pos + 1) == Some(&b'u')
        {
            let saved = self.pos;
            self.pos += 2;
            let second = self.hex4()?;
            if (0xdc00..0xe000).contains(&second) {
                let combined = 0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00);
                return Ok(char::from_u32(combined).unwrap_or('\u{fffd}'));
            }
            self.pos = saved;
        }
        Ok(char::from_u32(first).unwrap_or('\u{fffd}'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strict_json_parses_like_serde() {
        let text = r#"{"a": [1, -2.5, "x\"y\u00e9", true, null], "b": {"c": "\ud83d\ude00"}}"#;
        assert_eq!(
            parse(text).unwrap(),
            serde_json::from_str::<Value>(text).unwrap()
        );
    }

    #[test]
    fn newtonsoft_leniencies_are_accepted() {
        let text = "\u{feff}{\n  // a comment\n  modid: 'mymod', /* block */\n  \"Version\": \"1.0.0\",\n  \"authors\": [\"A\", \"B\",],\n}\n";
        assert_eq!(
            parse(text).unwrap(),
            json!({ "modid": "mymod", "Version": "1.0.0", "authors": ["A", "B"] })
        );
    }

    #[test]
    fn keys_are_found_regardless_of_case() {
        let value = parse(r#"{"ModID": "a", "Name": "B"}"#).unwrap();
        assert_eq!(get(&value, "modid"), Some(&json!("a")));
        assert_eq!(get(&value, "NAME"), Some(&json!("B")));
        assert_eq!(get(&value, "version"), None);
    }

    #[test]
    fn broken_input_is_an_error() {
        for text in [
            "{",
            "{\"a\" 1}",
            "[1 2]",
            "{\"a\": tru}",
            "\"open",
            "{} x",
            "/* open",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn deep_nesting_is_refused() {
        let text = "[".repeat(200) + &"]".repeat(200);
        assert!(parse(&text).is_err());
    }
}
