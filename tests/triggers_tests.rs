//! Интеграционные тесты для движка триггеров.

use std::thread::sleep;
use std::time::Duration;

use rust_terminal::config::{SendFormat, TriggerRule};
use rust_terminal::triggers::TriggerEngine;

fn mk_rule(name: &str, pat: &str, cooldown_ms: u32) -> TriggerRule {
    TriggerRule {
        name: name.into(),
        pattern: pat.into(),
        response: "OK".into(),
        response_format: SendFormat::Ascii,
        add_newline: true,
        enabled: true,
        cooldown_ms,
        apply_to_tx: false,
    }
}

#[test]
fn trigger_fires_on_match() {
    let mut eng = TriggerEngine::new();
    let rules = vec![mk_rule("ping", "hello", 0)];
    let fired = eng.check(&rules, "hello world");
    assert_eq!(fired, vec![0]);
}

#[test]
fn trigger_skips_no_match() {
    let mut eng = TriggerEngine::new();
    let rules = vec![mk_rule("ping", "HELLO", 0)];
    let fired = eng.check(&rules, "hello world");
    assert!(fired.is_empty());
}

#[test]
fn trigger_respects_cooldown() {
    let mut eng = TriggerEngine::new();
    let rules = vec![mk_rule("ping", "hello", 100)];
    let fired1 = eng.check(&rules, "hello");
    assert_eq!(fired1, vec![0]);
    let fired2 = eng.check(&rules, "hello again");
    assert!(fired2.is_empty());
    sleep(Duration::from_millis(120));
    let fired3 = eng.check(&rules, "hello");
    assert_eq!(fired3, vec![0]);
}

#[test]
fn disabled_trigger_ignored() {
    let mut eng = TriggerEngine::new();
    let mut rules = vec![mk_rule("ping", "hello", 0)];
    rules[0].enabled = false;
    assert!(eng.check(&rules, "hello").is_empty());
}

#[test]
fn invalid_regex_does_not_panic() {
    let mut eng = TriggerEngine::new();
    let rules = vec![mk_rule("bad", "(unclosed", 0)];
    assert!(eng.check(&rules, "text").is_empty());
    assert!(eng.regex_error("(unclosed").is_some());
}
