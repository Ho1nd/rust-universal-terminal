//! Инкрементальный поиск по активной панели лога.

use std::collections::VecDeque;

use regex::{Regex, RegexBuilder};

use crate::buffer::{LogLine, LogScope};
use crate::config::DisplayFormat;

#[derive(Default, Clone)]
pub struct SearchState {
    pub open: bool,
    pub query: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub scope: SearchScope,
    pub matches: Vec<usize>,
    pub current: Option<usize>,
    pub last_query: String,
    pub last_flags: u32,
    pub last_scope: SearchScope,
    pub last_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SearchScope {
    #[default]
    Rx,
    Tx,
    Combined,
}

impl SearchScope {
    pub fn to_log_scope(self) -> LogScope {
        match self {
            Self::Rx => LogScope::Rx,
            Self::Tx => LogScope::Tx,
            Self::Combined => LogScope::Combined,
        }
    }
}

impl SearchState {
    pub fn flags(&self) -> u32 {
        (self.case_sensitive as u32) | ((self.regex as u32) << 1)
    }

    pub fn reset(&mut self) {
        self.matches.clear();
        self.current = None;
        self.last_query.clear();
    }

    /// Пересчитать matches, если запрос/флаги/скоуп/данные поменялись.
    pub fn recompute(&mut self, lines: &VecDeque<LogLine>, fmt: DisplayFormat, escape: bool, revision: u64) {
        if self.query.is_empty() {
            self.matches.clear();
            self.current = None;
            self.last_query.clear();
            return;
        }
        let flags = self.flags();
        if self.last_query == self.query
            && self.last_flags == flags
            && self.last_scope == self.scope
            && self.last_revision == revision
            && !self.matches.is_empty()
        {
            return;
        }
        self.last_query = self.query.clone();
        self.last_flags = flags;
        self.last_scope = self.scope;
        self.last_revision = revision;
        self.matches.clear();

        let matcher = Matcher::build(&self.query, self.case_sensitive, self.regex);
        for (idx, line) in lines.iter().enumerate() {
            let s = line.formatted(fmt, escape);
            if matcher.matches(&s) {
                self.matches.push(idx);
            }
        }
        self.current = if self.matches.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn go_next(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let next = match self.current {
            None => 0,
            Some(i) => (i + 1) % self.matches.len(),
        };
        self.current = Some(next);
        Some(self.matches[next])
    }

    pub fn go_prev(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let next = match self.current {
            None => self.matches.len() - 1,
            Some(i) => {
                if i == 0 {
                    self.matches.len() - 1
                } else {
                    i - 1
                }
            }
        };
        self.current = Some(next);
        Some(self.matches[next])
    }
}

enum Matcher {
    Plain { needle: String, case_sensitive: bool },
    Re(Regex),
    Invalid,
}

impl Matcher {
    fn build(query: &str, case_sensitive: bool, regex: bool) -> Self {
        if regex {
            match RegexBuilder::new(query)
                .case_insensitive(!case_sensitive)
                .build()
            {
                Ok(re) => Self::Re(re),
                Err(_) => Self::Invalid,
            }
        } else {
            Self::Plain {
                needle: query.to_string(),
                case_sensitive,
            }
        }
    }

    fn matches(&self, text: &str) -> bool {
        match self {
            Self::Plain { needle, case_sensitive } => {
                if *case_sensitive {
                    text.contains(needle.as_str())
                } else {
                    text.to_lowercase().contains(&needle.to_lowercase())
                }
            }
            Self::Re(re) => re.is_match(text),
            Self::Invalid => false,
        }
    }
}
