use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table};
use ratatui::{Frame, Terminal};

use ipc_host::IpcCommands;
use ipc_host::value_table::Value;
use uipc_mapping::{FsuipcType, MappingSource};

fn f64_to_value(value: f64, ty: FsuipcType) -> Value {
    match ty {
        FsuipcType::U8 => Value::UnsignedInt8(value as u8),
        FsuipcType::I8 => Value::SignedInt8(value as i8),
        FsuipcType::U16 => Value::UnsignedInt16(value as u16),
        FsuipcType::I16 => Value::SignedInt16(value as i16),
        FsuipcType::U32 => Value::UnsignedInteger32(value as u32),
        FsuipcType::I32 => Value::SignedInt32(value as i32),
        FsuipcType::U64 => Value::UnsignedInt64(value as u64),
        FsuipcType::I64 => Value::Integer64(value as i64),
        FsuipcType::F32 => Value::Float32(value as f32),
        FsuipcType::F64 => Value::Float64(value),
        FsuipcType::String => Value::String(vec![0]),
    }
}

use crate::eval::{EvalEngine, MappingResult};
use crate::state;
use crate::trace::TraceBuffer;

pub struct App {
    engine: EvalEngine,
    results: Vec<MappingResult>,
    trace_buffer: Arc<Mutex<TraceBuffer>>,
    mapping_path: String,
    state_path: Option<String>,

    selected: usize,
    log_scroll: usize,
    log_autoscroll: bool,
    focus: Focus,
    log_visible: bool,

    show_help: bool,
    popup_index: Option<usize>,

    prompt: Option<Prompt>,
    should_quit: bool,

    ipc_handle: Option<JoinHandle<()>>,
    ipc_tx: Option<Sender<IpcCommands>>,
    ipc_enabled: bool,
    last_sync: Instant,
}

enum Focus {
    Table,
    Log,
}

struct Prompt {
    message: String,
    buffer: String,
    action: PromptAction,
}

enum PromptAction {
    ReloadMapping,
    LoadState,
    WriteState,
    WriteFsuipc,
}

impl App {
    pub fn new(
        mappings: Vec<uipc_mapping::DatarefMapping>,
        state: HashMap<String, f64>,
        mapping_path: String,
        trace_buffer: Arc<Mutex<TraceBuffer>>,
        ipc_handle: Option<JoinHandle<()>>,
        ipc_tx: Option<Sender<IpcCommands>>,
        ipc_enabled: bool,
    ) -> Self {
        let engine = EvalEngine::new(mappings, state);
        let results = engine.evaluate_all();
        Self {
            engine,
            results,
            trace_buffer,
            mapping_path,
            state_path: None,
            selected: 0,
            log_scroll: 0,
            log_autoscroll: true,
            focus: Focus::Table,
            log_visible: true,
            show_help: false,
            popup_index: None,
            prompt: None,
            should_quit: false,
            ipc_handle,
            ipc_tx,
            ipc_enabled,
            last_sync: Instant::now(),
        }
    }

    fn reload_eval(&mut self) {
        self.results = self.engine.evaluate_all();
        let missing = self.engine.missing_keys();
        if !missing.is_empty() {
            tracing::warn!("Missing state keys: {}", missing.join(", "));
        }
        tracing::info!("Evaluated {} mappings", self.results.len());
        self.sync_table();
        if let Some(tx) = &self.ipc_tx {
            tx.send(IpcCommands::ResetWarnings).unwrap();
        }
    }

    fn sync_table(&self) {
        if !self.ipc_enabled {
            return;
        }
        use ipc_host::value_table::{Entry, create_table_with_entries, set_value_table};
        let entries: Vec<(u16, Entry)> = self
            .results
            .iter()
            .filter_map(|r| {
                let value = r.fsuipc_value?;
                let entry = Entry {
                    value: f64_to_value(value, r.fsuipc_type),
                    source: 0,
                    destination: 0,
                    writable: r.writable,
                };
                Some((r.offset, entry))
            })
            .collect();
        let table = create_table_with_entries(&entries);
        set_value_table(table);
    }

    fn load_state(&mut self, path: &str) {
        match state::load_state(path) {
            Ok(new_state) => {
                let count = new_state.len();
                self.engine.state = new_state;
                self.state_path = Some(path.to_string());
                self.reload_eval();
                tracing::info!("Loaded state: {} entries from {}", count, path);
            }
            Err(e) => {
                tracing::error!("Failed to load state: {}", e);
            }
        }
    }

    fn write_state(&self, path: &str) {
        let keys = self.engine.all_referenced_keys();
        match state::write_state(path, &self.engine.state, &keys) {
            Ok(()) => {
                tracing::info!(
                    "Wrote state ({}/{} keys) to {}",
                    keys.len(),
                    keys.len(),
                    path
                );
            }
            Err(e) => {
                tracing::error!("Failed to write state: {}", e);
            }
        }
    }

    fn write_fsuipc(&self, path: &str) {
        match state::write_fsuipc_output(path, &self.results) {
            Ok(()) => {
                tracing::info!("Wrote {} mappings to {}", self.results.len(), path);
            }
            Err(e) => {
                tracing::error!("Failed to write FSUIPC output: {}", e);
            }
        }
    }
}

fn source_summary(source: &MappingSource) -> String {
    match source {
        MappingSource::Simple {
            dataref_path,
            scale,
            offset_add,
            ..
        } => {
            if *offset_add != 0.0 {
                format!("{} * {} + {}", dataref_path, scale, offset_add)
            } else if (*scale - 1.0).abs() > f64::EPSILON {
                format!("{} * {}", dataref_path, scale)
            } else {
                dataref_path.clone()
            }
        }
        MappingSource::Expr { expr, .. } => expr.to_string(),
        MappingSource::Static { static_value } => format!("static({})", static_value),
        MappingSource::StaticStr { static_str } => format!("static_str({})", static_str),
    }
}

fn fsuipc_type_str(ty: FsuipcType) -> &'static str {
    match ty {
        FsuipcType::I8 => "i8",
        FsuipcType::U8 => "u8",
        FsuipcType::I16 => "i16",
        FsuipcType::U16 => "u16",
        FsuipcType::I32 => "i32",
        FsuipcType::U32 => "u32",
        FsuipcType::I64 => "i64",
        FsuipcType::U64 => "u64",
        FsuipcType::F32 => "f32",
        FsuipcType::F64 => "f64",
        FsuipcType::String => "string",
    }
}

fn format_fsuipc_value(val: f64, ty: FsuipcType) -> String {
    match ty {
        FsuipcType::U32
        | FsuipcType::I32
        | FsuipcType::U16
        | FsuipcType::I16
        | FsuipcType::U8
        | FsuipcType::I8 => {
            format!("{}", val as i64)
        }
        FsuipcType::U64 | FsuipcType::I64 => {
            format!("{}", val as i64)
        }
        FsuipcType::F32 | FsuipcType::F64 => {
            format!("{:.4}", val)
        }
        FsuipcType::String => {
            format!("{}", val as u8 as char)
        }
    }
}

fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let has_expr = |idx: usize| -> bool {
        matches!(
            app.results.get(idx).map(|r| &r.source),
            Some(MappingSource::Expr { .. })
        )
    };

    let selected_style = Style::default().bg(Color::DarkGray).fg(Color::White);
    let expr_style = Style::default().fg(Color::Cyan);
    let marker = |idx: usize| -> &'static str { if has_expr(idx) { "▶" } else { " " } };

    let header = ["", "Offset", "Type", "W", "Inputs", "FSUIPC", "Source"];
    let widths = [
        Constraint::Length(2),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(3),
        Constraint::Min(20),
        Constraint::Length(14),
        Constraint::Min(10),
    ];

    let rows: Vec<Row> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let is_selected = i == app.selected && matches!(app.focus, Focus::Table);
            let mut style = if is_selected {
                selected_style
            } else {
                Style::default()
            };

            let val_str = match r.fsuipc_value {
                Some(v) => {
                    let s = format_fsuipc_value(v, r.fsuipc_type);
                    if r.inputs.iter().any(|inp| {
                        if let MappingSource::Simple { dataref_path, .. } = &r.source {
                            inp.0 == *dataref_path && !app.engine.state.contains_key(dataref_path)
                        } else {
                            false
                        }
                    }) {
                        style = style.fg(Color::Red);
                    }
                    s
                }
                None => {
                    style = style.fg(Color::Red);
                    "—".to_string()
                }
            };

            let writable = if r.writable { "rw" } else { "—" };

            let inputs_str = if r.inputs.is_empty() {
                "(static)".to_string()
            } else {
                r.inputs
                    .iter()
                    .map(|(key, _path, val)| format!("{}={}", key, val))
                    .collect::<Vec<_>>()
                    .join(" ")
            };

            let src = source_summary(&r.source);

            Row::new(vec![
                Cell::from(Span::styled(
                    marker(i),
                    if has_expr(i) {
                        expr_style
                    } else {
                        Style::default()
                    },
                )),
                Cell::from(format!("0x{:04X}", r.offset)),
                Cell::from(fsuipc_type_str(r.fsuipc_type)),
                Cell::from(writable),
                Cell::from(inputs_str),
                Cell::from(val_str),
                Cell::from(src),
            ])
            .style(style)
        })
        .collect();

    let title = if app.ipc_enabled {
        Span::styled(" Mappings (IPC) ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Mappings (Offline) ", Style::default())
    };

    let table = Table::new(rows, widths)
        .header(
            Row::new(header.iter().map(|h| {
                Cell::from(Span::styled(
                    *h,
                    Style::default().add_modifier(Modifier::BOLD),
                ))
            }))
            .bottom_margin(0),
        )
        .block(Block::default().title(title).borders(Borders::ALL))
        .row_highlight_style(selected_style)
        .highlight_symbol("");

    f.render_stateful_widget(
        table,
        area,
        &mut ratatui::widgets::TableState::new().with_selected(Some(app.selected)),
    );
}

fn render_log(f: &mut Frame, app: &mut App, area: Rect) {
    let entries: Vec<String> = match app.trace_buffer.lock() {
        Ok(buf) => buf.entries(),
        Err(_) => return,
    };

    let items: Vec<ListItem> = entries.iter().map(|e| ListItem::new(e.as_str())).collect();

    let list = List::new(items)
        .block(Block::default().title(" Trace Log ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = ratatui::widgets::ListState::default()
        .with_selected(Some(app.log_scroll.min(entries.len().saturating_sub(1))));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(""),
        Line::from("  Keybindings").bold(),
        Line::from("  ─────────────────────────────────────────────"),
        Line::from("  q        Quit"),
        Line::from("  ?        Toggle this help"),
        Line::from("  Tab      Cycle focus (Table ↔ Log)"),
        Line::from("  ↑/↓      Navigate table rows (table focus)"),
        Line::from("  PgUp/PgDn  Scroll log (log focus)"),
        Line::from("  End      Resume log auto-scroll"),
        Line::from("  Enter    Open expression detail popup"),
        Line::from("  Esc      Close popup"),
        Line::from("  r        Reload mapping file"),
        Line::from("  s        Load state CSV"),
        Line::from("  l        Toggle log pane"),
        Line::from("  w        Write state CSV (0-fill missing)"),
        Line::from("  c        Write computed FSUIPC values to CSV"),
        Line::from(""),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .alignment(ratatui::layout::Alignment::Left);

    let area = centered_rect(50, 60, area);
    f.render_widget(Clear, area);
    f.render_widget(help, area);
}

fn render_popup(f: &mut Frame, result: &MappingResult, area: Rect) {
    let expr_str = match &result.source {
        MappingSource::Expr { expr, .. } => expr.to_string(),
        _ => return,
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(format!(
            " Offset: 0x{:04X}  Type: {}",
            result.offset,
            fsuipc_type_str(result.fsuipc_type)
        ))
        .bold(),
        Line::from(""),
        Line::from(" Expression:"),
        Line::from(format!("  {}", expr_str)).fg(Color::Cyan),
        Line::from(""),
        Line::from(" Variables:"),
    ];

    for (name, path, val) in &result.inputs {
        lines.push(Line::from(format!("  ${}  {}  =  {}", name, path, val)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(" Press Esc to close").fg(Color::DarkGray));

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" Expression Detail ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)),
    );

    let area = centered_rect(70, 50, area);
    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}

fn render_prompt(f: &mut Frame, prompt: &Prompt, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(format!(" {}", prompt.message)).bold(),
        Line::from(""),
        Line::from(format!(" > {}", prompt.buffer)).fg(Color::Green),
        Line::from(""),
        Line::from(" Enter to confirm, Esc to cancel").fg(Color::DarkGray),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(" Input ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green)),
    );

    let area = centered_rect(60, 30, area);
    f.render_widget(Clear, area);
    f.render_widget(para, area);
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length((r.height.saturating_sub(height)) / 2),
                Constraint::Length(height),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length((r.width.saturating_sub(width)) / 2),
                Constraint::Length(width),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

pub fn run(mut app: App) -> Result<(), String> {
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).map_err(|e| e.to_string())?;

    let terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))
        .map_err(|e| e.to_string())?;

    let res = run_loop(terminal, &mut app);

    let mut stdout = std::io::stdout();
    execute!(stdout, LeaveAlternateScreen, cursor::Show).map_err(|e| e.to_string())?;
    terminal::disable_raw_mode().map_err(|e| e.to_string())?;

    res
}

fn run_loop(
    mut terminal: Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        terminal.draw(|f| ui(f, app)).map_err(|e| e.to_string())?;

        if app.should_quit {
            break;
        }

        if app.prompt.is_some() {
            if !handle_prompt_input(app) {
                break;
            }
        } else if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
            if !handle_normal_input(app) {
                break;
            }
        } else if app.ipc_enabled && app.last_sync.elapsed() >= Duration::from_millis(50) {
            app.sync_table();
            app.last_sync = Instant::now();
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    if app.show_help {
        render_help(f, f.area());
        return;
    }

    if let Some(idx) = app.popup_index {
        if let Some(result) = app.results.get(idx) {
            render_popup(f, result, f.area());
            return;
        }
    }

    let area = f.area();
    let chunks = if app.log_visible {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(10)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3)])
            .split(area)
    };

    render_table(f, app, chunks[0]);

    if app.log_visible {
        render_log(f, app, chunks[1]);
    }

    if let Some(ref prompt) = app.prompt {
        render_prompt(f, prompt, area);
    }
}

fn handle_prompt_input(app: &mut App) -> bool {
    if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
        return true;
    }

    let ev = event::read().unwrap_or(Event::Key(KeyCode::Esc.into()));
    match ev {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Esc => {
                let msg = match app.prompt.as_ref().unwrap().action {
                    PromptAction::ReloadMapping => "Mapping reload cancelled",
                    PromptAction::LoadState => "State load cancelled",
                    PromptAction::WriteState => "State write cancelled",
                    PromptAction::WriteFsuipc => "FSUIPC output cancelled",
                };
                tracing::info!("{}", msg);
                app.prompt = None;
            }
            KeyCode::Enter => {
                let action = app.prompt.take();
                if let Some(p) = action {
                    let path = p.buffer.trim().to_string();
                    if path.is_empty() {
                        tracing::warn!("No path entered, action cancelled");
                        return true;
                    }
                    match p.action {
                        PromptAction::ReloadMapping => match uipc_mapping::load_mappings(&path) {
                            Ok(config) => {
                                app.engine.mappings = config.mappings;
                                app.mapping_path = path.clone();
                                app.reload_eval();
                                tracing::info!(
                                    "Reloaded {} mappings from {}",
                                    app.engine.mappings.len(),
                                    path
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to reload mappings: {}", e);
                            }
                        },
                        PromptAction::LoadState => {
                            app.load_state(&path);
                        }
                        PromptAction::WriteState => {
                            app.write_state(&path);
                        }
                        PromptAction::WriteFsuipc => {
                            app.write_fsuipc(&path);
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut p) = app.prompt {
                    p.buffer.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut p) = app.prompt {
                    p.buffer.push(c);
                }
            }
            _ => {}
        },
        _ => {}
    }
    true
}

fn handle_normal_input(app: &mut App) -> bool {
    let ev = match event::read() {
        Ok(e) => e,
        Err(_) => return true,
    };

    match ev {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => {
                if let Some(tx) = app.ipc_tx.take() {
                    let _ = tx.send(IpcCommands::Shutdown);
                }
                if let Some(handle) = app.ipc_handle.take() {
                    let _ = handle.join();
                    tracing::info!("IPC thread joined");
                }
                app.should_quit = true;
            }
            KeyCode::Char('?') => {
                app.show_help = !app.show_help;
            }
            KeyCode::Tab => {
                app.focus = match app.focus {
                    Focus::Table if app.log_visible => Focus::Log,
                    _ => Focus::Table,
                };
            }
            KeyCode::Char('l') => {
                app.log_visible = !app.log_visible;
            }
            KeyCode::Char('r') => {
                app.prompt = Some(Prompt {
                    message: "Enter mapping file path:".to_string(),
                    buffer: app.mapping_path.clone(),
                    action: PromptAction::ReloadMapping,
                });
            }
            KeyCode::Char('s') => {
                let default = app.state_path.clone().unwrap_or_default();
                app.prompt = Some(Prompt {
                    message: "Enter state CSV path to load:".to_string(),
                    buffer: default,
                    action: PromptAction::LoadState,
                });
            }
            KeyCode::Char('w') => {
                let default = app
                    .state_path
                    .clone()
                    .unwrap_or_else(|| "state.csv".to_string());
                app.prompt = Some(Prompt {
                    message: "Enter output path for state CSV:".to_string(),
                    buffer: default,
                    action: PromptAction::WriteState,
                });
            }
            KeyCode::Char('c') => {
                app.prompt = Some(Prompt {
                    message: "Enter output path for FSUIPC values CSV:".to_string(),
                    buffer: "fsuipc_output.csv".to_string(),
                    action: PromptAction::WriteFsuipc,
                });
            }
            KeyCode::Up => {
                if matches!(app.focus, Focus::Table) && !app.results.is_empty() {
                    if app.selected > 0 {
                        app.selected -= 1;
                    }
                }
            }
            KeyCode::Down => {
                if matches!(app.focus, Focus::Table) && !app.results.is_empty() {
                    if app.selected + 1 < app.results.len() {
                        app.selected += 1;
                    }
                }
            }
            KeyCode::PageUp => {
                if matches!(app.focus, Focus::Log) {
                    app.log_scroll = app.log_scroll.saturating_sub(5);
                    app.log_autoscroll = false;
                }
            }
            KeyCode::PageDown => {
                if matches!(app.focus, Focus::Log) {
                    app.log_scroll += 5;
                    app.log_autoscroll = false;
                }
            }
            KeyCode::End => {
                if matches!(app.focus, Focus::Log) {
                    app.log_autoscroll = true;
                    if let Ok(buf) = app.trace_buffer.lock() {
                        app.log_scroll = buf.len().saturating_sub(1);
                    }
                }
            }
            KeyCode::Enter => {
                if matches!(app.focus, Focus::Table) {
                    if matches!(
                        app.results.get(app.selected).map(|r| &r.source),
                        Some(MappingSource::Expr { .. })
                    ) {
                        app.popup_index = Some(app.selected);
                    }
                }
            }
            KeyCode::Esc => {
                app.popup_index = None;
                app.show_help = false;
            }
            _ => {}
        },
        Event::Resize(_, _) => {}
        _ => {}
    }
    true
}
