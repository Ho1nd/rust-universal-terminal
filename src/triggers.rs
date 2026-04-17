//! Триггеры RX → TX. Regex-кэш и cooldown.

use std::collections::HashMap;
use std::time::Instant;

use regex::Regex;

use crate::config::TriggerRule;

pub struct TriggerEngine {
    regex_cache: HashMap<String, Option<Regex>>,
    last_fired: HashMap<usize, Instant>,
    fire_count: HashMap<usize, u32>,
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self {
            regex_cache: HashMap::new(),
            last_fired: HashMap::new(),
            fire_count: HashMap::new(),
        }
    }

    pub fn clear_cache(&mut self) {
        self.regex_cache.clear();
    }

    pub fn fire_count(&self, idx: usize) -> u32 {
        *self.fire_count.get(&idx).unwrap_or(&0)
    }

    pub fn last_fire_instant(&self, idx: usize) -> Option<Instant> {
        self.last_fired.get(&idx).copied()
    }

    /// Проверяет, сработает ли регекс на заданном тексте (без изменения
    /// состояния cooldown/счётчиков). Удобно для UI-превью.
    pub fn test_match(&mut self, pattern: &str, sample: &str) -> bool {
        if pattern.is_empty() {
            return false;
        }
        match self.get_regex(pattern) {
            Some(re) => re.is_match(sample),
            None => false,
        }
    }

    fn get_regex(&mut self, pat: &str) -> Option<&Regex> {
        if !self.regex_cache.contains_key(pat) {
            let compiled = Regex::new(pat).ok();
            self.regex_cache.insert(pat.to_string(), compiled);
        }
        self.regex_cache.get(pat).and_then(|o| o.as_ref())
    }

    /// Проверить строку RX против всех включённых правил. Вернёт индексы
    /// правил, которые сработали (cooldown уважён).
    pub fn check(&mut self, rules: &[TriggerRule], rx_line: &str) -> Vec<usize> {
        let now = Instant::now();
        let mut fired = Vec::new();
        for (i, rule) in rules.iter().enumerate() {
            if !rule.enabled || rule.pattern.is_empty() {
                continue;
            }
            let cooldown = std::time::Duration::from_millis(rule.cooldown_ms as u64);
            if let Some(prev) = self.last_fired.get(&i) {
                if now.duration_since(*prev) < cooldown {
                    continue;
                }
            }
            let re_matches = {
                let re = self.get_regex(&rule.pattern);
                re.map(|r| r.is_match(rx_line)).unwrap_or(false)
            };
            if re_matches {
                self.last_fired.insert(i, now);
                *self.fire_count.entry(i).or_insert(0) += 1;
                fired.push(i);
            }
        }
        fired
    }

    pub fn regex_error(&mut self, pat: &str) -> Option<String> {
        if pat.is_empty() {
            return None;
        }
        match Regex::new(pat) {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        }
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}
