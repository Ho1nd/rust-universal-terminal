//! `TerminalApp` — корень приложения, `eframe::App`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::Context;
use parking_lot::Mutex;

use crate::buffer::{Direction, LineAssembler, LogLine, LogScope, LogStore};
use crate::config::{AppConfig, DisplayFormat, SendFormat};
use crate::connection::{ConnectionManager, InboundMessage};
use crate::formats;
use crate::highlight::HighlightEngine;
use crate::persistence::ContinuousLogger;
use crate::scheduler::Scheduler;
use crate::search::SearchState;
use crate::triggers::TriggerEngine;
use crate::ui;

/// Единое транзитное сообщение для полос состояния/тостов.
#[derive(Clone)]
pub struct Toast {
    pub text: String,
    pub expires: Instant,
    pub is_error: bool,
}

/// Трекер пропускной способности (EMA за ~1с).
pub struct Throughput {
    rate_bps: f64,
    last_update: Instant,
    accumulator: u64,
}

impl Default for Throughput {
    fn default() -> Self {
        Self::new()
    }
}

impl Throughput {
    pub fn new() -> Self {
        Self {
            rate_bps: 0.0,
            last_update: Instant::now(),
            accumulator: 0,
        }
    }
    pub fn add(&mut self, n: u64) {
        self.accumulator = self.accumulator.saturating_add(n);
    }
    pub fn tick(&mut self) {
        let elapsed = self.last_update.elapsed();
        if elapsed >= Duration::from_millis(250) {
            let seconds = elapsed.as_secs_f64().max(1e-6);
            let instant_rate = self.accumulator as f64 / seconds;
            let alpha = 0.3;
            self.rate_bps = self.rate_bps * (1.0 - alpha) + instant_rate * alpha;
            self.accumulator = 0;
            self.last_update = Instant::now();
        }
    }
    pub fn bytes_per_sec(&self) -> f64 {
        self.rate_bps
    }
}

pub struct TerminalApp {
    pub config: AppConfig,
    pub log_store: Arc<Mutex<LogStore>>,
    pub connection: ConnectionManager,

    pub rx_assembler: LineAssembler,
    pub tx_assembler: LineAssembler,

    pub scheduler: Scheduler,
    pub triggers: TriggerEngine,
    pub highlight_engine: HighlightEngine,
    pub search: SearchState,

    pub rx_throughput: Throughput,
    pub tx_throughput: Throughput,
    pub connected_since: Option<Instant>,

    pub send_input: String,
    pub send_format: SendFormat,
    pub add_newline: bool,
    pub interpret_escapes: bool,
    pub show_sent: bool,
    pub history_index: Option<usize>,
    pub history_draft: String,

    pub toast: Option<Toast>,
    pub active_scope: LogScope,
    pub paused: bool,
    pub com_ports: Vec<String>,

    pub continuous_logger: Option<Arc<ContinuousLogger>>,

    pub config_dirty_since: Option<Instant>,

    pub flash_line: Option<(LogScope, usize, Instant)>,
    /// Запрос на прокрутку к строке в указанной панели. Потребляется один раз
    /// при рендеринге, после чего сбрасывается.
    pub scroll_request: Option<(LogScope, usize)>,
}

impl TerminalApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut config: AppConfig) -> Self {
        config.migrate_scheduled_commands_json();
        ui::theme_colors::install_system_fonts(&cc.egui_ctx);
        ui::theme_colors::apply_theme(&cc.egui_ctx, &config);
        ui::theme_colors::apply_font(&cc.egui_ctx, &config);

        let mut rx_asm = LineAssembler::new(Direction::Rx);
        let mut tx_asm = LineAssembler::new(Direction::Tx);
        rx_asm.set_mode(config.display.mode);
        tx_asm.set_mode(config.display.mode);
        rx_asm.set_limits(
            config.display.idle_flush_ms,
            config.display.max_line_bytes,
            config.display.timer_interval_ms,
        );
        tx_asm.set_limits(
            config.display.idle_flush_ms,
            config.display.max_line_bytes,
            config.display.timer_interval_ms,
        );
        rx_asm.set_split_on_literal_escapes(config.display.split_on_literal_escapes);
        tx_asm.set_split_on_literal_escapes(config.display.split_on_literal_escapes);

        let continuous_logger = config
            .continuous_log_file
            .as_ref()
            .and_then(|p| match ContinuousLogger::open(p.clone()) {
                Ok(l) => Some(Arc::new(l)),
                Err(e) => {
                    log::warn!("cannot open continuous log {}: {e}", p.display());
                    None
                }
            });

        let mut app = Self {
            log_store: Arc::new(Mutex::new(LogStore::new(config.max_log_lines))),
            connection: ConnectionManager::new(),
            rx_assembler: rx_asm,
            tx_assembler: tx_asm,
            scheduler: Scheduler::new(),
            triggers: TriggerEngine::new(),
            highlight_engine: HighlightEngine::new(),
            search: SearchState::default(),
            rx_throughput: Throughput::new(),
            tx_throughput: Throughput::new(),
            connected_since: None,

            send_input: String::new(),
            send_format: SendFormat::Ascii,
            add_newline: false,
            interpret_escapes: true,
            show_sent: true,
            history_index: None,
            history_draft: String::new(),

            toast: None,
            active_scope: LogScope::Rx,
            paused: false,
            com_ports: list_ports(),
            continuous_logger,
            config_dirty_since: None,
            flash_line: None,
            scroll_request: None,
            config,
        };
        app.refresh_scheduler_if_needed();
        app
    }

    pub fn mark_config_dirty(&mut self) {
        self.config_dirty_since = Some(Instant::now());
    }

    pub fn show_toast(&mut self, text: impl Into<String>, is_error: bool) {
        self.toast = Some(Toast {
            text: text.into(),
            expires: Instant::now() + Duration::from_secs(4),
            is_error,
        });
    }

    pub fn refresh_scheduler_if_needed(&mut self) {
        if self.scheduler.running {
            self.scheduler.start(&self.config.scheduled_commands);
        }
    }

    pub fn refresh_ports(&mut self) {
        self.com_ports = list_ports();
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }

    pub fn try_connect(&mut self) {
        match self.connection.connect(&self.config.last_connection) {
            Ok(()) => {
                self.connected_since = Some(Instant::now());
                self.rx_assembler.reset();
                self.tx_assembler.reset();
                self.mark_config_dirty();
            }
            Err(e) => {
                self.show_toast(format!("Ошибка подключения: {e}"), true);
                let err = LogLine::error(&format!("Ошибка подключения: {e}"));
                self.log_store.lock().push(err);
            }
        }
    }

    pub fn disconnect(&mut self) {
        if self.connection.is_connected() {
            self.connection.disconnect();
        }
        self.connected_since = None;
        if let Some(line) = self.rx_assembler.flush_now() {
            self.log_store.lock().push(line);
        }
        if let Some(line) = self.tx_assembler.flush_now() {
            self.log_store.lock().push(line);
        }
        if self.scheduler.running {
            self.scheduler.stop();
        }
    }

    pub fn send_raw(&mut self, data: Vec<u8>) {
        if !self.is_connected() {
            self.show_toast("Подключение не установлено", true);
            return;
        }
        if let Err(e) = self.connection.send(data) {
            self.show_toast(format!("Ошибка отправки: {e}"), true);
        }
    }

    /// Парсит текст по заданному формату + флагу CR/LF и отправляет.
    ///
    /// `interpret_escapes` включает распознавание `\n`, `\r`, `\t`, `\\`, `\0`,
    /// `\xHH` в ASCII-формате (в остальных форматах флаг игнорируется).
    pub fn send_text(&mut self, text: &str, fmt: SendFormat, add_nl: bool, interpret_escapes: bool) {
        if text.is_empty() {
            return;
        }
        match formats::parse_payload_opts(text, fmt, add_nl, interpret_escapes) {
            Ok(data) => self.send_raw(data),
            Err(e) => self.show_toast(format!("Парсинг: {e}"), true),
        }
    }

    /// Пушит в историю отправки (до 200 элементов).
    pub fn push_history(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.config.send_history.back().map(|s| s.as_str()) == Some(text.as_str()) {
            return;
        }
        self.config.send_history.push_back(text);
        while self.config.send_history.len() > 200 {
            self.config.send_history.pop_front();
        }
    }

    pub fn default_format_for(&self, scope: LogScope) -> DisplayFormat {
        match scope {
            LogScope::Rx => self.config.display.rx_format,
            LogScope::Tx => self.config.display.tx_format,
            LogScope::Combined => self.config.display.combined_format,
        }
    }

    pub fn set_format_for(&mut self, scope: LogScope, fmt: DisplayFormat) {
        match scope {
            LogScope::Rx => self.config.display.rx_format = fmt,
            LogScope::Tx => self.config.display.tx_format = fmt,
            LogScope::Combined => self.config.display.combined_format = fmt,
        }
        self.mark_config_dirty();
    }

    /// Обработка одного чанка RX: сборка, подсветка, триггеры, tee.
    fn process_rx(&mut self, chunk: Vec<u8>) {
        self.rx_throughput.add(chunk.len() as u64);
        self.rx_assembler.set_mode(self.config.display.mode);
        self.rx_assembler.set_limits(
            self.config.display.idle_flush_ms,
            self.config.display.max_line_bytes,
            self.config.display.timer_interval_ms,
        );
        self.rx_assembler
            .set_split_on_literal_escapes(self.config.display.split_on_literal_escapes);
        let raw_lines = self.rx_assembler.feed(&chunk);
        // Страховочный сплит по \n/\r/\r\n на случай любых пропусков в ассемблере
        // или режима Raw: пользователь всегда хочет, чтобы переводы строк были
        // физическими разрывами в логе.
        let split_literal = self.config.display.split_on_literal_escapes;
        let lines: Vec<LogLine> = raw_lines
            .into_iter()
            .flat_map(|l| split_logline_on_newlines(l, split_literal))
            .collect();
        for mut line in lines {
            self.apply_highlight(&mut line);
            if let Some(lg) = &self.continuous_logger {
                lg.log(&line);
            }
            let ascii_text = line.formatted(DisplayFormat::Ascii, false);
            self.log_store.lock().push(line);
            self.fire_triggers_for_line(Direction::Rx, &ascii_text);
        }
    }

    fn process_tx(&mut self, chunk: Vec<u8>) {
        self.tx_throughput.add(chunk.len() as u64);
        self.tx_assembler.set_mode(self.config.display.mode);
        self.tx_assembler.set_limits(
            self.config.display.idle_flush_ms,
            self.config.display.max_line_bytes,
            self.config.display.timer_interval_ms,
        );
        self.tx_assembler
            .set_split_on_literal_escapes(self.config.display.split_on_literal_escapes);
        let raw_lines = self.tx_assembler.feed(&chunk);
        let split_literal = self.config.display.split_on_literal_escapes;
        let lines: Vec<LogLine> = raw_lines
            .into_iter()
            .flat_map(|l| split_logline_on_newlines(l, split_literal))
            .collect();
        for mut line in lines {
            self.apply_highlight(&mut line);
            if let Some(lg) = &self.continuous_logger {
                lg.log(&line);
            }
            let ascii_text = line.formatted(DisplayFormat::Ascii, false);
            self.log_store.lock().push(line);
            self.fire_triggers_for_line(Direction::Tx, &ascii_text);
        }
    }

    /// Общая точка проверки и срабатывания триггеров. Для TX учитываются только
    /// правила с `apply_to_tx = true` (по умолчанию выключено — семантика
    /// «авто-ответчик на RX» сохраняется).
    fn fire_triggers_for_line(&mut self, direction: Direction, ascii_text: &str) {
        let rules: Vec<_> = if direction == Direction::Rx {
            self.config.triggers.clone()
        } else {
            self.config
                .triggers
                .iter()
                .map(|r| {
                    let mut rr = r.clone();
                    // Для TX пропускаем правила без apply_to_tx, занулив enabled.
                    if !rr.apply_to_tx {
                        rr.enabled = false;
                    }
                    rr
                })
                .collect()
        };
        if rules.iter().all(|r| !r.enabled) {
            return;
        }
        let fired = self.triggers.check(&rules, ascii_text);
        for idx in fired {
            let Some(rule) = self.config.triggers.get(idx).cloned() else { continue };
            // В ответе триггера тоже интерпретируем escape-последовательности —
            // так пользователь может задать «1000\n» и получить реальный перевод
            // строки, а не 5 литеральных байт.
            let parsed = formats::parse_payload_opts(
                &rule.response,
                rule.response_format,
                rule.add_newline,
                true,
            );
            match parsed {
                Ok(data) => {
                    self.send_raw(data);
                    let msg = format!("Триггер «{}» сработал", rule.name);
                    self.log_store.lock().push(LogLine::info(&msg));
                    self.show_toast(&msg, false);
                }
                Err(e) => {
                    let msg = format!("Триггер «{}» — ошибка ответа: {e}", rule.name);
                    self.log_store.lock().push(LogLine::error(&msg));
                    self.show_toast(&msg, true);
                }
            }
        }
    }

    fn apply_highlight(&mut self, line: &mut LogLine) {
        if self.config.highlights.is_empty() {
            return;
        }
        let text = line.formatted(DisplayFormat::Ascii, false);
        if let Some(color) = self
            .highlight_engine
            .color_for(&self.config.highlights, line.direction, &text)
        {
            line.highlight = Some(color);
        }
    }

    fn drain_inbound(&mut self) {
        let rx = self.connection.inbound();
        let mut rx_buf: Vec<u8> = Vec::new();
        let mut tx_buf: Vec<u8> = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            match msg {
                InboundMessage::RxBytes(data) => rx_buf.extend_from_slice(&data),
                InboundMessage::TxEcho(data) => tx_buf.extend_from_slice(&data),
                InboundMessage::Connected(desc) => {
                    self.log_store
                        .lock()
                        .push(LogLine::info(&format!("Подключено: {desc}")));
                    self.connected_since = Some(Instant::now());
                }
                InboundMessage::Disconnected(reason) => {
                    let text = match reason {
                        Some(r) => format!("Отключено: {r}"),
                        None => "Отключено".into(),
                    };
                    self.log_store.lock().push(LogLine::info(&text));
                    self.connected_since = None;
                    if let Some(line) = self.rx_assembler.flush_now() {
                        self.log_store.lock().push(line);
                    }
                    if let Some(line) = self.tx_assembler.flush_now() {
                        self.log_store.lock().push(line);
                    }
                }
                InboundMessage::Error(e) => {
                    self.log_store.lock().push(LogLine::error(&e));
                }
            }
        }
        if !rx_buf.is_empty() {
            self.process_rx(rx_buf);
        }
        if !tx_buf.is_empty() {
            self.process_tx(tx_buf);
        }
    }

    fn poll_assemblers_timeout(&mut self) {
        for line in self.rx_assembler.poll_timeout() {
            let mut l = line;
            self.apply_highlight(&mut l);
            if let Some(lg) = &self.continuous_logger {
                lg.log(&l);
            }
            self.log_store.lock().push(l);
        }
        for line in self.tx_assembler.poll_timeout() {
            let mut l = line;
            self.apply_highlight(&mut l);
            if let Some(lg) = &self.continuous_logger {
                lg.log(&l);
            }
            self.log_store.lock().push(l);
        }
    }

    fn tick_scheduler(&mut self) {
        if !self.scheduler.running {
            return;
        }
        let commands = self.config.scheduled_commands.clone();
        let fire = self.scheduler.tick(&commands);
        for i in fire {
            if let Some(cmd) = commands.get(i) {
                match formats::parse_payload(&cmd.payload, cmd.format, cmd.add_newline) {
                    Ok(data) => self.send_raw(data),
                    Err(e) => self.show_toast(
                        format!("Команда '{}' парсинг: {e}", cmd.name),
                        true,
                    ),
                }
            }
        }
    }

    fn autosave_config_if_needed(&mut self) {
        if let Some(t) = self.config_dirty_since {
            if t.elapsed() >= Duration::from_secs(2) {
                self.config.save();
                self.config_dirty_since = None;
            }
        }
    }

    /// Сбросить кэш форматирования у всех уже накопленных строк.
    pub fn invalidate_format_cache(&self) {
        let store = self.log_store.lock();
        for line in store.rx.iter().chain(store.tx.iter()).chain(store.combined.iter()) {
            line.invalidate_cache();
        }
    }

    pub fn set_continuous_log(&mut self, path: Option<std::path::PathBuf>) {
        self.continuous_logger = match &path {
            Some(p) => match ContinuousLogger::open(p.clone()) {
                Ok(l) => Some(Arc::new(l)),
                Err(e) => {
                    self.show_toast(format!("Не удалось открыть файл: {e}"), true);
                    None
                }
            },
            None => None,
        };
        self.config.continuous_log_file = path;
        self.mark_config_dirty();
    }
}

impl eframe::App for TerminalApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.drain_inbound();
        self.poll_assemblers_timeout();
        self.tick_scheduler();

        self.rx_throughput.tick();
        self.tx_throughput.tick();

        ui::render_app(self, ctx);

        if let Some(t) = &self.toast {
            if Instant::now() >= t.expires {
                self.toast = None;
            }
        }

        self.autosave_config_if_needed();

        // Постоянный repaint — на частоте 30 fps достаточно для терминала
        ctx.request_repaint_after(Duration::from_millis(33));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.disconnect();
        self.config.save();
    }
}

/// Разбивает одну строку лога на несколько, если её байты содержат `\n`,
/// `\r` или `\r\n`. Порядок и направление сохраняются. Пустые подстроки между
/// разделителями сохраняются как пустые строки — это корректно отражает
/// «пустые строки» в исходном потоке (например, подряд идущие CRLF).
/// При `literal_escapes = true` дополнительно режет по литеральным
/// последовательностям `\n`, `\r`, `\r\n` (байты `\\` + `n`/`r`).
fn split_logline_on_newlines(line: LogLine, literal_escapes: bool) -> Vec<LogLine> {
    let bytes: &[u8] = &line.bytes;
    let has_any = bytes.iter().any(|&b| b == b'\n' || b == b'\r')
        || (literal_escapes && contains_literal_escape(bytes));
    if !has_any {
        return vec![line];
    }

    let mut out: Vec<LogLine> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            out.push(LogLine::new(
                line.direction,
                line.timestamp,
                Arc::from(&bytes[start..i]),
            ));
            i += 1;
            start = i;
        } else if b == b'\r' {
            out.push(LogLine::new(
                line.direction,
                line.timestamp,
                Arc::from(&bytes[start..i]),
            ));
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
            start = i;
        } else if literal_escapes && b == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'n' => {
                    let end = if i >= 2 && &bytes[i - 2..i] == b"\\r" {
                        i - 2
                    } else {
                        i
                    };
                    out.push(LogLine::new(
                        line.direction,
                        line.timestamp,
                        Arc::from(&bytes[start..end]),
                    ));
                    i += 2;
                    start = i;
                }
                b'r' => {
                    // Если это `\r` без следующего `\n`, режем сразу.
                    let is_crlf = i + 3 < bytes.len() && &bytes[i + 2..i + 4] == b"\\n";
                    if !is_crlf {
                        out.push(LogLine::new(
                            line.direction,
                            line.timestamp,
                            Arc::from(&bytes[start..i]),
                        ));
                        i += 2;
                        start = i;
                    } else {
                        // Дадим следующей итерации обработать `\n` как CRLF.
                        i += 2;
                    }
                }
                _ => i += 1,
            }
        } else {
            i += 1;
        }
    }
    if start < bytes.len() {
        out.push(LogLine::new(
            line.direction,
            line.timestamp,
            Arc::from(&bytes[start..]),
        ));
    }
    if out.is_empty() {
        out.push(LogLine::new(
            line.direction,
            line.timestamp,
            Arc::from(&[][..]),
        ));
    }
    out
}

fn contains_literal_escape(bytes: &[u8]) -> bool {
    bytes
        .windows(2)
        .any(|w| w[0] == b'\\' && (w[1] == b'n' || w[1] == b'r'))
}

fn list_ports() -> Vec<String> {
    match serialport::available_ports() {
        Ok(ports) => {
            let mut out: Vec<String> = ports
                .into_iter()
                .map(|p| p.port_name)
                .collect::<Vec<_>>();
            out.sort();
            out.dedup();
            out
        }
        Err(e) => {
            log::warn!("list_ports: {e}");
            Vec::new()
        }
    }
}
