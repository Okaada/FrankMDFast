//! A tiny JSON codec with JavaScript semantics (`JSON.parse` / `JSON.stringify`).
//!
//! Records must round-trip byte-for-byte with what the original wrote, so the
//! parser keeps object keys in source order (duplicate keys: last value wins,
//! at the first key's position — like `JSON.parse`), numbers are IEEE doubles,
//! and the serializer reproduces `JSON.stringify` exactly: no whitespace,
//! non-ASCII and `/` unescaped, integral numbers without a fraction.
//!
//! Deviation: Rust strings cannot hold lone UTF-16 surrogates, so a `\uD800`
//! escape without its pair decodes to U+FFFD.

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Object),
}

/// An insertion-ordered JSON object.
pub(crate) type Object = Vec<(String, Json)>;

impl Json {
    /// The object's entries, if this is an object.
    pub(crate) fn as_object(&self) -> Option<&Object> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }
}

/// Look up `key` in an ordered object.
pub(crate) fn get<'a>(obj: &'a Object, key: &str) -> Option<&'a Json> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `obj[key]` as a string (`typeof === "string"`).
pub(crate) fn get_str<'a>(obj: &'a Object, key: &str) -> Option<&'a str> {
    match get(obj, key) {
        Some(Json::String(s)) => Some(s),
        _ => None,
    }
}

/// `obj[key]` as a finite number (`Number.isFinite`).
pub(crate) fn get_finite(obj: &Object, key: &str) -> Option<f64> {
    match get(obj, key) {
        Some(Json::Number(n)) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// `obj[key] === 1`.
pub(crate) fn is_one(obj: &Object, key: &str) -> bool {
    matches!(get(obj, key), Some(Json::Number(n)) if *n == 1.0)
}

/// Set `key` in place (keeping its position) or append it, like JS assignment.
pub(crate) fn set(obj: &mut Object, key: &str, value: Json) {
    if let Some(slot) = obj.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        obj.push((key.to_owned(), value));
    }
}

// ---------------------------------------------------------------------------
// Parse
// ---------------------------------------------------------------------------

/// `JSON.parse(text)`; `None` where the original would throw a `SyntaxError`.
pub(crate) fn parse(text: &str) -> Option<Json> {
    let mut p = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let value = p.value(0)?;
    p.skip_ws();
    (p.pos == p.bytes.len()).then_some(value)
}

/// Nesting limit so hostile input cannot overflow the stack.
const MAX_DEPTH: usize = 512;

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Option<Json> {
        let end = self.pos.checked_add(word.len())?;
        if self.bytes.get(self.pos..end)? == word.as_bytes() {
            self.pos = end;
            Some(value)
        } else {
            None
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > MAX_DEPTH {
            return None;
        }
        match self.peek()? {
            b'n' => self.literal("null", Json::Null),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'"' => self.string().map(Json::String),
            b'[' => self.array(depth),
            b'{' => self.object(depth),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn array(&mut self, depth: usize) -> Option<Json> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.eat(b']') {
            return Some(Json::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            if self.eat(b']') {
                return Some(Json::Array(items));
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }

    fn object(&mut self, depth: usize) -> Option<Json> {
        self.pos += 1;
        let mut entries = Object::new();
        self.skip_ws();
        if self.eat(b'}') {
            return Some(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return None;
            }
            let key = self.string()?;
            self.skip_ws();
            if !self.eat(b':') {
                return None;
            }
            self.skip_ws();
            let value = self.value(depth + 1)?;
            set(&mut entries, &key, value);
            self.skip_ws();
            if self.eat(b'}') {
                return Some(Json::Object(entries));
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.pos;
        self.eat(b'-');
        match self.peek()? {
            b'0' => self.pos += 1,
            b'1'..=b'9' => {
                self.digits();
            }
            _ => return None,
        }
        if self.eat(b'.') && self.digits() == 0 {
            return None;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if self.digits() == 0 {
                return None;
            }
        }
        let lexeme = std::str::from_utf8(self.bytes.get(start..self.pos)?).ok()?;
        // Rust's float parsing is correctly rounded (overflow → ±inf), like JS.
        lexeme.parse::<f64>().ok().map(Json::Number)
    }

    fn hex4(&mut self) -> Option<u16> {
        let end = self.pos.checked_add(4)?;
        let digits = std::str::from_utf8(self.bytes.get(self.pos..end)?).ok()?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        self.pos = end;
        u16::from_str_radix(digits, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        self.pos += 1; // opening quote
        let mut units: Vec<u16> = Vec::new();
        let mut out = String::new();
        loop {
            let start = self.pos;
            // Copy a run of plain characters at once.
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            if self.pos > start {
                flush_units(&mut units, &mut out);
                out.push_str(std::str::from_utf8(self.bytes.get(start..self.pos)?).ok()?);
            }
            match self.bump()? {
                b'"' => {
                    flush_units(&mut units, &mut out);
                    return Some(out);
                }
                b'\\' => {
                    let unit = match self.bump()? {
                        b'"' => u16::from(b'"'),
                        b'\\' => u16::from(b'\\'),
                        b'/' => u16::from(b'/'),
                        b'b' => 0x08,
                        b'f' => 0x0c,
                        b'n' => u16::from(b'\n'),
                        b'r' => u16::from(b'\r'),
                        b't' => u16::from(b'\t'),
                        b'u' => self.hex4()?,
                        _ => return None,
                    };
                    units.push(unit);
                }
                // Unescaped control character.
                _ => return None,
            }
        }
    }
}

/// Decode pending escaped UTF-16 units (pairs surrogates; lone ones → U+FFFD).
fn flush_units(units: &mut Vec<u16>, out: &mut String) {
    if !units.is_empty() {
        out.extend(
            char::decode_utf16(units.drain(..)).map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER)),
        );
    }
}

// ---------------------------------------------------------------------------
// Stringify
// ---------------------------------------------------------------------------

/// `JSON.stringify(value)`.
pub(crate) fn stringify(value: &Json) -> String {
    let mut out = String::new();
    write_value(&mut out, value);
    out
}

/// `JSON.stringify` of an ordered object.
pub(crate) fn stringify_object(obj: &[(String, Json)]) -> String {
    let mut out = String::new();
    write_object(&mut out, obj);
    out
}

fn write_value(out: &mut String, value: &Json) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => {
            if n.is_finite() {
                out.push_str(&js_number(*n));
            } else {
                out.push_str("null");
            }
        }
        Json::String(s) => write_string(out, s),
        Json::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Json::Object(obj) => write_object(out, obj),
    }
}

fn write_object(out: &mut String, obj: &[(String, Json)]) {
    out.push('{');
    for (i, (k, v)) in obj.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_string(out, k);
        out.push(':');
        write_value(out, v);
    }
    out.push('}');
}

/// `JSON.stringify` string quoting (`QuoteJSONString`).
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// JavaScript `Number.prototype.toString()` (radix 10), e.g. `1700000000000`,
/// `1.5`, `1e+21`, `1e-7`, `Infinity`.
pub(crate) fn js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    // Shortest round-trip digits in scientific form, e.g. "1.5e0".
    let sci = format!("{:e}", n.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let point = exp + 1; // position of the decimal point relative to the digits

    let body = if k <= point && point <= 21 {
        let zeros = usize::try_from(point - k).unwrap_or(0);
        format!("{digits}{}", "0".repeat(zeros))
    } else if 0 < point && point <= 21 {
        let split = usize::try_from(point).unwrap_or(0);
        format!("{}.{}", &digits[..split], &digits[split..])
    } else if -6 < point && point <= 0 {
        let zeros = usize::try_from(-point).unwrap_or(0);
        format!("0.{}{digits}", "0".repeat(zeros))
    } else {
        let e = point - 1;
        let e_sign = if e < 0 { '-' } else { '+' };
        let (first, rest) = digits.split_at(1);
        if rest.is_empty() {
            format!("{first}e{e_sign}{}", e.abs())
        } else {
            format!("{first}.{rest}e{e_sign}{}", e.abs())
        }
    };
    format!("{sign}{body}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn numbers_format_like_javascript() {
        for (n, s) in [
            (1_700_000_000_000.0, "1700000000000"),
            (1.5, "1.5"),
            (-2.0, "-2"),
            (0.1, "0.1"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (0.000001, "0.000001"),
            (123.456, "123.456"),
            (1.2345e25, "1.2345e+25"),
            (-0.0, "0"),
        ] {
            assert_eq!(js_number(n), s);
        }
    }

    #[test]
    fn parse_keeps_order_and_round_trips() {
        let raw = r#"{"b":1,"a":[true,null,"xé\n"],"b":2.5,"c":{"z":1,"y":"/é"}}"#;
        let v = parse(raw).unwrap();
        assert_eq!(
            stringify(&v),
            r#"{"b":2.5,"a":[true,null,"xé\n"],"c":{"z":1,"y":"/é"}}"#
        );
        assert!(parse("{bad").is_none());
        assert!(parse("01").is_none());
        assert!(parse("").is_none());
        assert!(parse(" [1, 2] ").is_some());
        assert_eq!(parse(r#""😀""#), Some(Json::String("😀".into())));
        assert_eq!(parse(r#""\ud800""#), Some(Json::String("\u{fffd}".into())));
    }
}
