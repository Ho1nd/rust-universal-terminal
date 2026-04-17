# Rust Terminal

Кроссплатформенный (Windows + Linux) UART/TCP/UDP терминал на Rust + eframe/egui. Rust-порт Python/PyQt-приложения (`../uart_terminal.py`).

## Возможности

- **Подключения**: UART (полная настройка baud/data/stop/parity/flow), TCP-клиент, UDP (клиент + опциональный bind на локальный порт).
- **Пять форматов отображения**: ASCII, HEX, Decimal, Binary, Mixed (hex-dump + ASCII).
- **Сборка строк** из входящего потока тремя способами: по `\n`/`\r\n` (LF и CRLF), по таймеру или raw-чанками. Таймаут простоя flush’ит хвост.
- **Split-view RX / TX** с независимыми форматами, драг-разделитель, combined-view по Ctrl+M.
- **Планировщик** периодических команд с ограничением числа повторений.
- **Макросы F1..F12** (с модификаторами Ctrl/Shift/Alt).
- **Триггеры** RX → TX по regex с cooldown.
- **Подсветка** строк по regex с выбором цвета.
- **Поиск** (Ctrl+F) инкрементальный, с опциями Aa / regex, подсветка найденной строки.
- **Темы**: Dark, Light, Solarized Dark, Dracula, Custom.
- **Статус-бар**: скорость RX/TX (EMA ~1с), суммарный объём, uptime.
- **Сохранение лога** в `.txt` (в выбранном формате) и `.csv` (timestamp, direction, hex, ascii).
- **Непрерывная запись** (tee) в файл с flush после каждой строки.
- **Пресеты** подключений; **история** команд (Up/Down в поле отправки).
- **Кольцевой буфер** до 100 000 строк (настраиваемо); виртуализированный рендер.
- **Импорт** `scheduled_commands.json` от Python-версии при первом запуске.

## Сборка

### Windows

```powershell
# Rust stable 1.75+
rustup default stable
cargo build --release
# исполняемый файл: target\release\rust-terminal.exe
```

### Linux

```bash
# Зависимости (Debian/Ubuntu): libudev — для serialport, gtk для rfd
sudo apt install build-essential libudev-dev libgtk-3-dev pkg-config
cargo build --release
# исполняемый файл: target/release/rust-terminal
```

### Запуск тестов

```bash
cargo test
```

Покрыты: `LineAssembler` (LF/CRLF/mixed, таймаут, max_line_bytes, raw/timer-режимы), `formats` (форматирование и парсинг всех форматов отправки), `triggers` (срабатывание, cooldown, disabled, невалидный regex).

## Горячие клавиши

| Клавиша | Действие |
| --- | --- |
| `Ctrl+F` | Открыть/закрыть поиск |
| `Esc` | Закрыть поиск |
| `Enter` / `Shift+Enter` в поиске | Следующее / предыдущее совпадение |
| `Ctrl+B` | Показать/скрыть левую панель |
| `Ctrl+M` | Combined view ↔ split |
| `Ctrl+L` | Очистить активную панель |
| `Ctrl+S` | Сохранить лог активной панели |
| `Ctrl+Enter` в поле отправки | Отправить |
| `↑` / `↓` в поле отправки | История команд |
| `F1..F12` (+ Ctrl/Shift/Alt) | Отправить макрос (если фокус не в поле ввода) |

## Где хранится конфигурация

- Windows: `%APPDATA%\RustTerminal\config.json`
- Linux: `$XDG_CONFIG_HOME/rust-terminal/config.json` (обычно `~/.config/rust-terminal/config.json`)

## Структура проекта

См. `../RUST_PORT_SPEC.md` — полное ТЗ с описанием архитектуры.

```
src/
├── app.rs              # TerminalApp (eframe::App)
├── lib.rs / main.rs
├── config/             # AppConfig, темы, пресеты, модели
├── connection/         # trait Connection + UART/TCP/UDP + manager (IO-поток)
├── buffer/             # LineAssembler, LogStore, LogLine
├── formats.rs          # форматирование + парсинг
├── scheduler.rs        # периодические команды
├── macros.rs           # F1..F12
├── triggers.rs         # RX → TX regex + cooldown
├── highlight.rs        # подсветка
├── search.rs           # Ctrl+F
├── persistence.rs      # save log + continuous-tee
└── ui/                 # egui-панели
```

## Лицензия

MIT
