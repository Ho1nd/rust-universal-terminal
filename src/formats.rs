//! Форматы отображения и парсинг отправки.

use std::fmt::Write as _;

use crate::config::{DisplayFormat, SendFormat};

/// Форматирует срез байт в соответствии с форматом отображения.
pub fn format_bytes(data: &[u8], fmt: DisplayFormat, escape_control: bool) -> String {
    match fmt {
        DisplayFormat::Ascii => ascii(data, escape_control),
        DisplayFormat::Hex => hex_spaced(data),
        DisplayFormat::Decimal => decimal_spaced(data),
        DisplayFormat::Binary => binary_spaced(data),
        DisplayFormat::Mixed => mixed(data),
    }
}

fn ascii(data: &[u8], escape: bool) -> String {
    match std::str::from_utf8(data) {
        Ok(s) => {
            if !escape {
                s.to_string()
            } else {
                let mut out = String::with_capacity(s.len() + 4);
                for ch in s.chars() {
                    match ch {
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        '\0' => out.push_str("\\0"),
                        c if (c as u32) < 0x20 => {
                            let _ = write!(out, "\\x{:02X}", c as u32);
                        }
                        c => out.push(c),
                    }
                }
                out
            }
        }
        Err(_) => {
            let mut out = String::with_capacity(data.len());
            for &b in data {
                if escape {
                    match b {
                        b'\n' => out.push_str("\\n"),
                        b'\r' => out.push_str("\\r"),
                        b'\t' => out.push_str("\\t"),
                        0 => out.push_str("\\0"),
                        0x20..=0x7e => out.push(b as char),
                        _ => {
                            let _ = write!(out, "\\x{:02X}", b);
                        }
                    }
                } else if (0x20..=0x7e).contains(&b) {
                    out.push(b as char);
                } else {
                    out.push('.');
                }
            }
            out
        }
    }
}

fn hex_spaced(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 3);
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{b:02X}"));
    }
    out
}

fn decimal_spaced(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4);
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&b.to_string());
    }
    out
}

fn binary_spaced(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 9);
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{b:08b}"));
    }
    out
}

fn mixed(data: &[u8]) -> String {
    let hex = hex_spaced(data);
    let mut ascii = String::with_capacity(data.len());
    for &b in data {
        if (0x20..=0x7e).contains(&b) {
            ascii.push(b as char);
        } else {
            ascii.push('.');
        }
    }
    format!("{hex:<40} | {ascii}")
}

/// Ошибки парсинга отправляемых данных.
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("HEX строка должна содержать чётное количество символов")]
    HexOddLength,
    #[error("Некорректный HEX: {0}")]
    HexInvalid(String),
    #[error("Некорректное число в десятичном формате: {0}")]
    DecimalInvalid(String),
    #[error("Десятичное значение вне диапазона 0..=255: {0}")]
    DecimalOutOfRange(i64),
    #[error("Binary: число бит должно быть кратно 8 (получено {0})")]
    BinaryNotMultipleOfEight(usize),
    #[error("Binary: допустимы только символы 0 и 1")]
    BinaryInvalidChar,
    #[error("Некорректная escape-последовательность: {0}")]
    InvalidEscape(String),
    #[error("Пустые данные")]
    Empty,
}

pub fn parse_payload(text: &str, fmt: SendFormat, add_newline: bool) -> Result<Vec<u8>, ParseError> {
    parse_payload_opts(text, fmt, add_newline, false)
}

/// Расширенная версия `parse_payload`: при `interpret_escapes=true` в ASCII
/// распознаёт `\n`, `\r`, `\t`, `\0`, `\\`, `\"`, `\'`, `\xHH`.
pub fn parse_payload_opts(
    text: &str,
    fmt: SendFormat,
    add_newline: bool,
    interpret_escapes: bool,
) -> Result<Vec<u8>, ParseError> {
    let mut data = parse_payload_raw(text, fmt, interpret_escapes)?;
    if add_newline {
        data.extend_from_slice(b"\r\n");
    }
    Ok(data)
}

fn parse_payload_raw(text: &str, fmt: SendFormat, interpret_escapes: bool) -> Result<Vec<u8>, ParseError> {
    match fmt {
        SendFormat::Ascii => {
            if text.is_empty() {
                return Err(ParseError::Empty);
            }
            if interpret_escapes {
                unescape_ascii(text)
            } else {
                Ok(text.as_bytes().to_vec())
            }
        }
        SendFormat::Hex => {
            let cleaned: String = text
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .replace("0x", "")
                .replace("0X", "");
            if cleaned.is_empty() {
                return Err(ParseError::Empty);
            }
            if cleaned.len() % 2 != 0 {
                return Err(ParseError::HexOddLength);
            }
            hex::decode(&cleaned).map_err(|e| ParseError::HexInvalid(e.to_string()))
        }
        SendFormat::Decimal => {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.is_empty() {
                return Err(ParseError::Empty);
            }
            let mut out = Vec::with_capacity(parts.len());
            for p in parts {
                let n: i64 = p.parse().map_err(|_| ParseError::DecimalInvalid(p.to_string()))?;
                if !(0..=255).contains(&n) {
                    return Err(ParseError::DecimalOutOfRange(n));
                }
                out.push(n as u8);
            }
            Ok(out)
        }
        SendFormat::Binary => {
            let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            if cleaned.is_empty() {
                return Err(ParseError::Empty);
            }
            if !cleaned.chars().all(|c| c == '0' || c == '1') {
                return Err(ParseError::BinaryInvalidChar);
            }
            if cleaned.len() % 8 != 0 {
                return Err(ParseError::BinaryNotMultipleOfEight(cleaned.len()));
            }
            let mut out = Vec::with_capacity(cleaned.len() / 8);
            for chunk in cleaned.as_bytes().chunks(8) {
                let s = std::str::from_utf8(chunk).unwrap_or("00000000");
                let byte = u8::from_str_radix(s, 2).map_err(|_| ParseError::BinaryInvalidChar)?;
                out.push(byte);
            }
            Ok(out)
        }
    }
}

/// Разворачивает популярные C-подобные escape-последовательности в байты.
fn unescape_ascii(text: &str) -> Result<Vec<u8>, ParseError> {
    let mut out: Vec<u8> = Vec::with_capacity(text.len());
    let mut it = text.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match it.next() {
            Some('n') => out.push(b'\n'),
            Some('r') => out.push(b'\r'),
            Some('t') => out.push(b'\t'),
            Some('0') => out.push(0),
            Some('a') => out.push(0x07),
            Some('b') => out.push(0x08),
            Some('f') => out.push(0x0C),
            Some('v') => out.push(0x0B),
            Some('e') => out.push(0x1B),
            Some('\\') => out.push(b'\\'),
            Some('"') => out.push(b'"'),
            Some('\'') => out.push(b'\''),
            Some('x') => {
                let h1 = it.next().ok_or_else(|| ParseError::InvalidEscape("\\x".into()))?;
                let h2 = it.next().ok_or_else(|| ParseError::InvalidEscape(format!("\\x{h1}")))?;
                let hex_str: String = [h1, h2].iter().collect();
                let byte = u8::from_str_radix(&hex_str, 16)
                    .map_err(|_| ParseError::InvalidEscape(format!("\\x{hex_str}")))?;
                out.push(byte);
            }
            Some(other) => {
                return Err(ParseError::InvalidEscape(format!("\\{other}")));
            }
            None => return Err(ParseError::InvalidEscape("\\".into())),
        }
    }
    if out.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_format_uppercase_spaced() {
        assert_eq!(format_bytes(b"Hello", DisplayFormat::Hex, false), "48 65 6C 6C 6F");
    }

    #[test]
    fn decimal_format_works() {
        assert_eq!(format_bytes(b"AB", DisplayFormat::Decimal, false), "65 66");
    }

    #[test]
    fn binary_format_8bit() {
        assert_eq!(format_bytes(&[0x48], DisplayFormat::Binary, false), "01001000");
    }

    #[test]
    fn ascii_escape_newline() {
        let s = format_bytes(b"a\nb", DisplayFormat::Ascii, true);
        assert_eq!(s, "a\\nb");
    }

    #[test]
    fn ascii_escape_cr_tab_null() {
        let s = format_bytes(b"a\rb\tc\0d", DisplayFormat::Ascii, true);
        assert_eq!(s, "a\\rb\\tc\\0d");
    }

    #[test]
    fn ascii_escape_low_control_hex() {
        let s = format_bytes(&[0x01, b'A', 0x1B], DisplayFormat::Ascii, true);
        assert_eq!(s, "\\x01A\\x1B");
    }

    #[test]
    fn mixed_pad() {
        let s = format_bytes(b"Hello", DisplayFormat::Mixed, false);
        assert!(s.contains("48 65 6C 6C 6F"));
        assert!(s.contains("| Hello"));
    }

    #[test]
    fn parse_hex_ok() {
        assert_eq!(parse_payload("48 65", SendFormat::Hex, false).unwrap(), b"He".to_vec());
        assert_eq!(parse_payload("0x4865", SendFormat::Hex, false).unwrap(), b"He".to_vec());
    }

    #[test]
    fn parse_hex_odd_err() {
        assert!(parse_payload("ABC", SendFormat::Hex, false).is_err());
    }

    #[test]
    fn parse_decimal_range() {
        assert!(parse_payload("256", SendFormat::Decimal, false).is_err());
        assert_eq!(parse_payload("72 101", SendFormat::Decimal, false).unwrap(), vec![72, 101]);
    }

    #[test]
    fn parse_binary_ok() {
        assert_eq!(parse_payload("01001000 01100101", SendFormat::Binary, false).unwrap(), b"He".to_vec());
    }

    #[test]
    fn parse_binary_not_multiple_of_8() {
        assert!(parse_payload("0100100", SendFormat::Binary, false).is_err());
    }

    #[test]
    fn add_newline_crlf() {
        let d = parse_payload("A", SendFormat::Ascii, true).unwrap();
        assert_eq!(d, b"A\r\n".to_vec());
    }

    #[test]
    fn escape_newline_ascii() {
        let d = parse_payload_opts("123\\n123\\n123", SendFormat::Ascii, false, true).unwrap();
        assert_eq!(d, b"123\n123\n123".to_vec());
    }

    #[test]
    fn escape_hex_byte() {
        let d = parse_payload_opts("\\x48\\x49", SendFormat::Ascii, false, true).unwrap();
        assert_eq!(d, b"HI".to_vec());
    }

    #[test]
    fn escape_literal_backslash() {
        let d = parse_payload_opts("a\\\\b", SendFormat::Ascii, false, true).unwrap();
        assert_eq!(d, b"a\\b".to_vec());
    }

    #[test]
    fn escape_disabled_passes_through() {
        let d = parse_payload_opts("a\\nb", SendFormat::Ascii, false, false).unwrap();
        assert_eq!(d, b"a\\nb".to_vec());
    }
}
