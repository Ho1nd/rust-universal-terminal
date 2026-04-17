//! Правила подсветки. Подсветка применяется к конкретным фрагментам строки
//! (не ко всей строке целиком). При перекрытии первое правило побеждает.

use std::collections::HashMap;

use regex::Regex;

use crate::buffer::Direction;
use crate::config::HighlightRule;

/// Один раскрашенный фрагмент строки: байтовые индексы (на char-границах).
#[derive(Clone, Copy, Debug)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub color: [u8; 4],
    pub bold: bool,
}

pub struct HighlightEngine {
    cache: HashMap<String, Option<Regex>>,
}

impl HighlightEngine {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    fn get_regex(&mut self, pat: &str) -> Option<&Regex> {
        if !self.cache.contains_key(pat) {
            self.cache.insert(pat.to_string(), Regex::new(pat).ok());
        }
        self.cache.get(pat).and_then(|o| o.as_ref())
    }

    /// Возвращает набор непересекающихся подсвеченных фрагментов в `line_text`.
    /// Правила проверяются по порядку; в перекрывающихся байтах выигрывает
    /// правило, встреченное раньше.
    pub fn spans_for(
        &mut self,
        rules: &[HighlightRule],
        direction: Direction,
        line_text: &str,
    ) -> Vec<HighlightSpan> {
        let mut spans: Vec<HighlightSpan> = Vec::new();
        for rule in rules {
            if !rule.enabled || rule.pattern.is_empty() {
                continue;
            }
            let ok_dir = match direction {
                Direction::Rx => rule.apply_to_rx,
                Direction::Tx => rule.apply_to_tx,
                _ => false,
            };
            if !ok_dir {
                continue;
            }
            let Some(re) = self.get_regex(&rule.pattern) else { continue; };
            for m in re.find_iter(line_text) {
                let (s, e) = (m.start(), m.end());
                if s == e {
                    continue;
                }
                if spans.iter().any(|sp| sp.start < e && s < sp.end) {
                    continue;
                }
                spans.push(HighlightSpan {
                    start: s,
                    end: e,
                    color: rule.color,
                    bold: rule.bold,
                });
            }
        }
        spans.sort_by_key(|sp| sp.start);
        spans
    }

    /// Устаревший метод для обратной совместимости: возвращает цвет первого
    /// сработавшего правила либо `None`. В основном рендеринге не используется.
    #[allow(dead_code)]
    pub fn color_for(
        &mut self,
        rules: &[HighlightRule],
        direction: Direction,
        line_text: &str,
    ) -> Option<[u8; 4]> {
        self.spans_for(rules, direction, line_text)
            .first()
            .map(|sp| sp.color)
    }
}

impl Default for HighlightEngine {
    fn default() -> Self {
        Self::new()
    }
}
