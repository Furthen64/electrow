#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().and_then(|n| {
            if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= u64::MAX as f64 {
                Some(n as u64)
            } else {
                None
            }
        })
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn has_non_finite(&self) -> bool {
        match self {
            Value::Number(number) => !number.is_finite(),
            Value::Array(items) => items.iter().any(Value::has_non_finite),
            Value::Object(entries) => entries.iter().any(|(_, value)| value.has_non_finite()),
            _ => false,
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out);
        out
    }
}

const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

fn utf8_width(byte: u8) -> Option<usize> {
    match byte {
        0x00..=0x7f => Some(1),
        0xc0..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf7 => Some(4),
        _ => None,
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    parser.whitespace();
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.pos == parser.bytes.len() {
        Ok(value)
    } else {
        Err(format!("unexpected trailing bytes at {}", parser.pos))
    }
}

impl<'a> Parser<'a> {
    fn whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        self.whitespace();
        let byte = *self.bytes.get(self.pos).ok_or("unexpected end of input")?;
        match byte {
            b'{' => self.object(depth + 1),
            b'[' => self.array(depth + 1),
            b'"' => self.string().map(Value::String),
            b't' => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            b'f' => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            b'n' => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(format!("unexpected byte {:?} at {}", byte as char, self.pos)),
        }
    }

    fn literal(&mut self, expected: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.pos..self.pos + expected.len()) == Some(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            Err(format!("invalid literal at {}", self.pos))
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        let number = text.parse::<f64>().map_err(|e| e.to_string())?;
        if !number.is_finite() {
            return Err(format!("non-finite number at {}", start));
        }
        Ok(Value::Number(number))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(format!("expected string at {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = *self.bytes.get(self.pos).ok_or("unterminated escape")?;
                    self.pos += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.hex4()?;
                            let ch = match code {
                                0xd800..=0xdbff => {
                                    if self.bytes.get(self.pos) != Some(&b'\\')
                                        || self.bytes.get(self.pos + 1) != Some(&b'u')
                                    {
                                        return Err(format!("unpaired high surrogate at {}", self.pos));
                                    }
                                    self.pos += 2;
                                    let low = self.hex4()?;
                                    if !(0xdc00..=0xdfff).contains(&low) {
                                        return Err(format!("invalid low surrogate at {}", self.pos));
                                    }
                                    let combined =
                                        0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                                    char::from_u32(combined).ok_or("invalid surrogate pair")?
                                }
                                0xdc00..=0xdfff => {
                                    return Err(format!("unpaired low surrogate at {}", self.pos));
                                }
                                _ => char::from_u32(code).unwrap_or('\u{fffd}'),
                            };
                            out.push(ch);
                        }
                        _ => return Err(format!("invalid escape at {}", self.pos)),
                    }
                }
                0..=0x1f => return Err(format!("control character in string at {}", self.pos)),
                _ => {
                    let start = self.pos - 1;
                    let width =
                        utf8_width(byte).ok_or_else(|| format!("invalid utf-8 at {start}"))?;
                    let end = start + width;
                    if end > self.bytes.len() {
                        return Err(format!("truncated utf-8 at {start}"));
                    }
                    let ch = std::str::from_utf8(&self.bytes[start..end])
                        .map_err(|e| e.to_string())?
                        .chars()
                        .next()
                        .ok_or("invalid utf-8")?;
                    out.push(ch);
                    self.pos = end;
                }
            }
        }
        Err("unterminated string".to_string())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err("truncated unicode escape".to_string());
        }
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = (self.bytes[self.pos] as char)
                .to_digit(16)
                .ok_or("invalid unicode escape")?;
            value = value * 16 + digit;
            self.pos += 1;
        }
        Ok(value)
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(format!("maximum nesting depth {MAX_DEPTH} exceeded at {}", self.pos));
        }
        self.pos += 1;
        let mut items = Vec::new();
        self.whitespace();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.whitespace();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(format!("expected ',' or ']' at {}", self.pos)),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(format!("maximum nesting depth {MAX_DEPTH} exceeded at {}", self.pos));
        }
        self.pos += 1;
        let mut entries = Vec::new();
        self.whitespace();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Value::Object(entries));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            self.whitespace();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(format!("expected ':' at {}", self.pos));
            }
            self.pos += 1;
            let value = self.value(depth)?;
            entries.push((key, value));
            self.whitespace();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(entries));
                }
                _ => return Err(format!("expected ',' or '}}' at {}", self.pos)),
            }
        }
    }
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => {
            if number.is_finite() {
                out.push_str(&number.to_string());
            } else {
                out.push_str("null");
            }
        }
        Value::String(text) => write_string(text, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(entries) => {
            out.push('{');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_value(value, out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_non_finite_values() {
        assert!(Value::Number(f64::NAN).has_non_finite());
        assert!(Value::Array(vec![Value::Number(f64::INFINITY)]).has_non_finite());
        assert!(Value::Object(vec![(
            "a".to_string(),
            Value::Number(f64::NEG_INFINITY)
        )])
        .has_non_finite());
        assert!(!Value::Object(vec![("a".to_string(), Value::Number(1.0))]).has_non_finite());
    }

    #[test]
    fn round_trip_nested_value() {
        let input = r#"{"v":1,"id":"a","args":{"text":"line\nquote\"","n":5e-6,"list":[true,null,-2]}}"#;
        let value = parse(input).unwrap();
        assert_eq!(value.get("v"), Some(&Value::Number(1.0)));
        assert_eq!(
            value.get("args").and_then(|a| a.get("text")),
            Some(&Value::String("line\nquote\"".to_string()))
        );
        let reparsed = parse(&value.to_json()).unwrap();
        assert_eq!(reparsed, value);
    }

    #[test]
    fn rejects_trailing_data() {
        assert!(parse("{} {}").is_err());
    }

    #[test]
    fn combines_surrogate_pairs_and_rejects_lone_surrogates() {
        let value = parse(r#"{"s":"\uD83D\uDE00"}"#).unwrap();
        assert_eq!(value.get("s"), Some(&Value::String("😀".to_string())));
        assert!(parse(r#""\uD83D""#).is_err());
        assert!(parse(r#""\uDE00""#).is_err());
    }

    #[test]
    fn parses_multibyte_utf8() {
        let value = parse("\"é€😀\"").unwrap();
        assert_eq!(value, Value::String("é€😀".to_string()));
    }

    #[test]
    fn rejects_excessive_nesting() {
        let input = format!("{}{}", "[".repeat(MAX_DEPTH + 2), "]".repeat(MAX_DEPTH + 2));
        assert!(parse(&input).is_err());
    }

    #[test]
    fn parses_large_string() {
        let text = "a".repeat(200_000);
        let value = parse(&format!("\"{text}\"")).unwrap();
        assert_eq!(value, Value::String(text));
    }
}
