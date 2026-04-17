//! Интеграционные тесты для форматов и парсинга.

use rust_terminal::config::{DisplayFormat, SendFormat};
use rust_terminal::formats::{format_bytes, parse_payload};

#[test]
fn hex_mixed_format() {
    let s = format_bytes(b"Hi!", DisplayFormat::Mixed, false);
    assert!(s.contains("48 69 21"));
    assert!(s.contains("| Hi!"));
}

#[test]
fn hex_parser_with_prefix_and_spaces() {
    let data = parse_payload("0x48 65", SendFormat::Hex, false).unwrap();
    assert_eq!(data, b"He".to_vec());
}

#[test]
fn decimal_parser_out_of_range() {
    assert!(parse_payload("10 20 300", SendFormat::Decimal, false).is_err());
}

#[test]
fn binary_parser_padding_required() {
    assert!(parse_payload("1111", SendFormat::Binary, false).is_err());
    let data = parse_payload("00001111", SendFormat::Binary, false).unwrap();
    assert_eq!(data, vec![0x0F]);
}

#[test]
fn add_newline_adds_crlf() {
    let d = parse_payload("A", SendFormat::Ascii, true).unwrap();
    assert_eq!(d, b"A\r\n".to_vec());
}
