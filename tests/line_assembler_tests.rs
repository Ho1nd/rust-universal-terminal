//! Интеграционные тесты для `LineAssembler`.

use std::thread::sleep;
use std::time::Duration;

use rust_terminal::buffer::{Direction, LineAssembler};
use rust_terminal::config::DisplayMode;

fn text_of(line: &rust_terminal::buffer::LogLine) -> String {
    String::from_utf8_lossy(&line.bytes).into_owned()
}

#[test]
fn unix_lf_splits_two_lines() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    let out = a.feed(b"hello\nworld\n");
    assert_eq!(out.len(), 2);
    assert_eq!(text_of(&out[0]), "hello");
    assert_eq!(text_of(&out[1]), "world");
}

#[test]
fn windows_crlf_stripped() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    let out = a.feed(b"hello\r\nworld\r\n");
    assert_eq!(out.len(), 2);
    assert_eq!(text_of(&out[0]), "hello");
    assert_eq!(text_of(&out[1]), "world");
}

#[test]
fn mixed_with_timeout_flushes_tail() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_limits(50, 4096, 50);
    let mut out = a.feed(b"a\nb\r\nc");
    assert_eq!(out.len(), 2);
    assert_eq!(text_of(&out[0]), "a");
    assert_eq!(text_of(&out[1]), "b");
    sleep(Duration::from_millis(80));
    out = a.poll_timeout();
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "c");
}

#[test]
fn byte_by_byte_input() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    assert!(a.feed(b"h").is_empty());
    assert!(a.feed(b"i").is_empty());
    let out = a.feed(b"\n");
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "hi");
}

#[test]
fn max_line_bytes_forced_flush() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_limits(200, 8, 50);
    let out = a.feed(b"0123456789"); // 10 bytes, > 8
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].bytes.len(), 10);
}

#[test]
fn timer_mode_emits_on_poll() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByTimer);
    a.set_limits(200, 4096, 20);
    assert!(a.feed(b"partial").is_empty());
    sleep(Duration::from_millis(40));
    let out = a.poll_timeout();
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "partial");
}

#[test]
fn user_scenario_three_lines_with_trailing_crlf() {
    // Точная ситуация пользователя: escape-обработка даёт 123\n123\n123,
    // плюс CR/LF-чекбокс добавляет \r\n в конце.
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    let out = a.feed(b"123\n123\n123\r\n");
    assert_eq!(out.len(), 3, "должно быть ровно 3 строки, получили {}: {:?}",
        out.len(),
        out.iter().map(text_of).collect::<Vec<_>>());
    assert_eq!(text_of(&out[0]), "123");
    assert_eq!(text_of(&out[1]), "123");
    assert_eq!(text_of(&out[2]), "123");
}

#[test]
fn user_scenario_three_lines_no_trailing_nl_flushes_on_timeout() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_limits(30, 4096, 50);
    let out = a.feed(b"123\n123\n123");
    assert_eq!(out.len(), 2);
    sleep(Duration::from_millis(50));
    let tail = a.poll_timeout();
    assert_eq!(tail.len(), 1);
    assert_eq!(text_of(&tail[0]), "123");
}

#[test]
fn bare_cr_splits_lines() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    // Хвостовой одиночный \r удерживается до следующего чанка (может быть CRLF).
    let out = a.feed(b"hello\rworld\rtail");
    assert_eq!(out.len(), 2);
    assert_eq!(text_of(&out[0]), "hello");
    assert_eq!(text_of(&out[1]), "world");
}

#[test]
fn mixed_cr_and_lf_each_split() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    // CR, LF, CRLF вперемешку
    let out = a.feed(b"1\r2\n3\r\n4\n");
    assert_eq!(out.len(), 4);
    assert_eq!(text_of(&out[0]), "1");
    assert_eq!(text_of(&out[1]), "2");
    assert_eq!(text_of(&out[2]), "3");
    assert_eq!(text_of(&out[3]), "4");
}

#[test]
fn trailing_cr_waits_for_next_byte() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    // \r в конце — подождать следующего чанка, возможно это CRLF.
    let out = a.feed(b"hi\r");
    assert!(out.is_empty());
    let out = a.feed(b"\n");
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "hi");
}

#[test]
fn raw_mode_each_chunk_is_line() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::Raw);
    let out = a.feed(b"abc");
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "abc");
}

#[test]
fn literal_backslash_n_splits_when_enabled() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(true);
    let out = a.feed(b"1\\n2\\n3\\n4\\n");
    assert_eq!(out.len(), 4);
    assert_eq!(text_of(&out[0]), "1");
    assert_eq!(text_of(&out[1]), "2");
    assert_eq!(text_of(&out[2]), "3");
    assert_eq!(text_of(&out[3]), "4");
}

#[test]
fn literal_backslash_n_not_split_when_disabled() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(false);
    a.set_limits(20, 4096, 50);
    let out = a.feed(b"2\\n3\n");
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "2\\n3");
}

#[test]
fn literal_crlf_splits_once() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(true);
    let out = a.feed(b"abc\\r\\ndef\\r\\n");
    assert_eq!(out.len(), 2);
    assert_eq!(text_of(&out[0]), "abc");
    assert_eq!(text_of(&out[1]), "def");
}

#[test]
fn literal_bare_cr_splits() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(true);
    let out = a.feed(b"a\\rb\\rc\\n");
    assert_eq!(out.len(), 3);
    assert_eq!(text_of(&out[0]), "a");
    assert_eq!(text_of(&out[1]), "b");
    assert_eq!(text_of(&out[2]), "c");
}

#[test]
fn literal_mixed_with_real_newlines() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(true);
    let out = a.feed(b"2\\n3\nhi\\r\\nok\n");
    assert_eq!(out.len(), 4);
    assert_eq!(text_of(&out[0]), "2");
    assert_eq!(text_of(&out[1]), "3");
    assert_eq!(text_of(&out[2]), "hi");
    assert_eq!(text_of(&out[3]), "ok");
}

#[test]
fn literal_trailing_backslash_waits_for_next_chunk() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(true);
    assert!(a.feed(b"hi\\").is_empty());
    let out = a.feed(b"n");
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "hi");
}

#[test]
fn literal_backslash_other_char_not_split() {
    let mut a = LineAssembler::new(Direction::Rx);
    a.set_mode(DisplayMode::ByNewline);
    a.set_split_on_literal_escapes(true);
    a.set_limits(20, 4096, 50);
    let out = a.feed(b"a\\tb\n");
    assert_eq!(out.len(), 1);
    assert_eq!(text_of(&out[0]), "a\\tb");
}
