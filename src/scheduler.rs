//! Планировщик периодических команд. Все таймеры — в UI-тике,
//! без дополнительных потоков.

use std::time::Instant;

use crate::config::ScheduledCommand;

#[derive(Default)]
pub struct SchedulerEntryState {
    pub last_fired: Option<Instant>,
    pub fired_count: u32,
}

pub struct Scheduler {
    pub running: bool,
    states: Vec<SchedulerEntryState>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            running: false,
            states: Vec::new(),
        }
    }

    pub fn start(&mut self, commands: &[ScheduledCommand]) {
        self.running = true;
        self.states.clear();
        self.states.resize_with(commands.len(), Default::default);
    }

    pub fn stop(&mut self) {
        self.running = false;
        self.states.clear();
    }

    /// Вернуть список индексов команд, которые нужно выполнить сейчас.
    /// Учитывает `interval_ms` и `repeat`.
    pub fn tick(&mut self, commands: &[ScheduledCommand]) -> Vec<usize> {
        if !self.running {
            return Vec::new();
        }
        if self.states.len() != commands.len() {
            self.states.resize_with(commands.len(), Default::default);
        }

        let now = Instant::now();
        let mut fire: Vec<usize> = Vec::new();
        for (i, cmd) in commands.iter().enumerate() {
            if !cmd.enabled {
                continue;
            }
            if cmd.repeat > 0 && self.states[i].fired_count >= cmd.repeat {
                continue;
            }
            let interval = std::time::Duration::from_millis(cmd.interval_ms as u64);
            let due = match self.states[i].last_fired {
                None => true,
                Some(t) => now.duration_since(t) >= interval,
            };
            if due {
                self.states[i].last_fired = Some(now);
                self.states[i].fired_count = self.states[i].fired_count.saturating_add(1);
                fire.push(i);
            }
        }
        fire
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
