use crate::consts::DB_PATH;
use crate::util::*;
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use rust_i18n::t;
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::widgets::*;
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Default, Hash)]
pub struct Trash {
    pub id: i64,
    pub time: i64,
    pub path: String,
    pub hash: String,
    pub size: String,
}
impl Trash {
    pub fn to_display(&self) -> Result<TrashDisplay> {
        let name = PathBuf::from(&self.path)
            .file_name()
            .with_context(|| t!("message.parse_name"))?
            .to_str()
            .unwrap_or_default()
            .to_string();
        let path = PathBuf::from(&self.path)
            .parent()
            .with_context(|| t!("message.parse_parent"))?
            .to_str()
            .unwrap_or_default()
            .to_string();
        let size = self.size.clone();
        let time = timestamp_human(self.time);
        let res = TrashDisplay {
            name,
            path,
            size,
            time,
        };
        Ok(res)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Default, Hash)]
pub struct TrashDisplay {
    name: String,
    path: String,
    size: String,
    time: String,
}
pub fn connect_database() -> Result<Connection> {
    let path = to_absolute_no_fs(DB_PATH);
    create_file_all(&path)?;
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r"
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS trash (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            time INTEGER NOT NULL,
            path TEXT NOT NULL,
            hash TEXT NOT NULL,
            size TEXT NOT NULL
        );
    ",
    )?;

    Ok(conn)
}

pub fn insert(conn: &mut Connection, args: &[Trash]) -> Result<()> {
    let tx = conn.transaction()?;
    let mut stmt = tx.prepare("INSERT INTO trash (time,path,hash,size) VALUES (?1, ?2,?3,?4)")?;
    for Trash {
        id: _,
        time,
        path,
        hash,
        size,
    } in args
    {
        stmt.execute(params![time, path, hash, size])?;
    }
    drop(stmt);
    tx.commit()?;
    Ok(())
}

pub fn delete(conn: &mut Connection, to_del: &[Trash]) -> Result<()> {
    let tx = conn.transaction()?;
    let mut stmt = tx.prepare("DELETE FROM trash WHERE id = ?1")?;
    for Trash {
        id,
        time: _,
        path: _,
        hash: _,
        size: _,
    } in to_del
    {
        stmt.execute(params![id])?;
    }
    drop(stmt);
    tx.commit()?;
    Ok(())
}

pub fn select_days_age(conn: &Connection, days: u16) -> Result<Vec<Trash>> {
    let time = days_ago(days);
    let mut stmt = conn.prepare("SELECT * FROM trash WHERE time < ?1")?;
    let all = stmt.query_map([time], |row| {
        Ok(Trash {
            id: row.get(0)?,
            time: row.get(1)?,
            path: row.get(2)?,
            hash: row.get(3)?,
            size: row.get(4)?,
        })
    })?;
    let mut res = Vec::new();

    for t in all {
        let item = t.with_context(|| t!("message.db_read_error"))?;
        res.push(item);
    }
    Ok(res)
}

pub fn exist_hash(conn: &Connection, hashes: &[String]) -> Result<Vec<bool>> {
    let mut stmt = conn.prepare("SELECT EXISTS(SELECT 1 FROM trash WHERE hash = ?1) AS exist")?;
    let mut res = Vec::new();
    for hash in hashes {
        let exist: u8 = stmt.query_row([hash], |row| row.get(0))?;
        if exist == 0 {
            res.push(false);
        } else {
            res.push(true);
        }
    }
    Ok(res)
}

// ─── TUI: select_visible ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum TuiMode {
    Normal,
    Searching,
    Help,
    Action,
}

#[derive(Debug, Clone, PartialEq)]
enum ActionChoice {
    Delete,
    Restore,
    Exit,
    Cancel,
}

impl ActionChoice {
    fn label(&self) -> String {
        match self {
            ActionChoice::Delete => t!("tui.del_sel").into_owned(),
            ActionChoice::Restore => t!("tui.rest_sel").into_owned(),
            ActionChoice::Exit => t!("tui.exit").into_owned(),
            ActionChoice::Cancel => t!("tui.cancel").into_owned(),
        }
    }

    fn all() -> Vec<ActionChoice> {
        vec![
            ActionChoice::Delete,
            ActionChoice::Restore,
            ActionChoice::Exit,
            ActionChoice::Cancel,
        ]
    }
}

struct TuiApp<'a> {
    items: Vec<(Trash, TrashDisplay)>,
    selected: HashSet<usize>,
    filtered: Vec<usize>,
    mode: TuiMode,
    table_state: TableState,
    search_query: String,
    action_cursor: usize,
    help_items: Vec<(&'a str, String)>,
}

impl<'a> TuiApp<'a> {
    fn new(items: Vec<Trash>) -> Result<Self> {
        let displays: Vec<(Trash, TrashDisplay)> = items
            .into_iter()
            .map(|t| {
                let d = t.to_display()?;
                Ok((t, d))
            })
            .collect::<Result<_>>()?;

        let filtered: Vec<usize> = (0..displays.len()).collect();
        let mut table_state = TableState::default();
        if !filtered.is_empty() {
            table_state.select(Some(0));
        }

        Ok(Self {
            items: displays,
            selected: HashSet::new(),
            filtered,
            mode: TuiMode::Normal,
            table_state,
            search_query: String::new(),
            action_cursor: 0,
            help_items: vec![
                ("\u{2191}/\u{2193}   j/k", t!("tui.move").into_owned()),
                ("PgUp/PgDn", t!("tui.scroll").into_owned()),
                ("Home / End", t!("tui.first_last").into_owned()),
                ("Space", t!("tui.toggle").into_owned()),
                ("a", t!("tui.sel_all").into_owned()),
                ("A", t!("tui.invert").into_owned()),
                ("/", t!("tui.search").into_owned()),
                ("Enter", t!("tui.action").into_owned()),
                ("Esc / q", t!("tui.exit_empty").into_owned()),
                ("?", t!("tui.help_toggle").into_owned()),
            ],
        })
    }

    fn visible_items(&self) -> Vec<&(Trash, TrashDisplay)> {
        self.filtered.iter().map(|&i| &self.items[i]).collect()
    }

    fn visible_selected(&self) -> Vec<usize> {
        self.filtered
            .iter()
            .filter(|&&i| self.selected.contains(&i))
            .copied()
            .collect()
    }

    fn current_original_idx(&self) -> Option<usize> {
        self.table_state.selected().map(|i| self.filtered[i])
    }

    fn select_next(&mut self) {
        let len = self.filtered.len();
        if len == 0 {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        if i + 1 < len {
            self.table_state.select(Some(i + 1));
        }
    }

    fn select_prev(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        if i > 0 {
            self.table_state.select(Some(i - 1));
        }
    }

    fn select_next_n(&mut self, n: usize) {
        let len = self.filtered.len();
        if len == 0 {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        let new = (i + n).min(len - 1);
        self.table_state.select(Some(new));
    }

    fn select_prev_n(&mut self, n: usize) {
        if self.filtered.is_empty() {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        let new = i.saturating_sub(n);
        self.table_state.select(Some(new));
    }

    fn go_top(&mut self) {
        if !self.filtered.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    fn go_bottom(&mut self) {
        let len = self.filtered.len();
        if len > 0 {
            self.table_state.select(Some(len - 1));
        }
    }

    fn page_up(&mut self) {
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some(i.saturating_sub(10)));
    }

    fn page_down(&mut self) {
        let len = self.filtered.len();
        if len == 0 {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some((i + 10).min(len - 1)));
    }

    fn toggle_selection(&mut self) {
        if let Some(orig) = self.current_original_idx() {
            if self.selected.contains(&orig) {
                self.selected.remove(&orig);
            } else {
                self.selected.insert(orig);
            }
        }
    }

    fn select_all(&mut self) {
        for &i in &self.filtered {
            self.selected.insert(i);
        }
    }

    fn invert_selection(&mut self) {
        for &i in &self.filtered {
            if self.selected.contains(&i) {
                self.selected.remove(&i);
            } else {
                self.selected.insert(i);
            }
        }
    }

    fn apply_search(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.filtered = (0..self.items.len()).collect();
        } else {
            self.filtered = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, (_, d))| {
                    d.name.to_lowercase().contains(&query) || d.path.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.filtered.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }
}

fn tui_render(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();

    let [main_area, status_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let rows: Vec<Row> = app
        .visible_items()
        .iter()
        .enumerate()
        .map(|(vi, (_trash, disp))| {
            let orig_idx = app.filtered[vi];
            let checked = if app.selected.contains(&orig_idx) {
                "[x]"
            } else {
                "[ ]"
            };
            let cells = vec![
                Cell::from(
                    Text::from(Line::from(Span::styled(
                        checked,
                        if app.selected.contains(&orig_idx) {
                            Style::default().fg(Color::Green).bold()
                        } else {
                            Style::default().dim()
                        },
                    )))
                    .alignment(Alignment::Center),
                ),
                Cell::from(Text::from(disp.name.as_str()).alignment(Alignment::Center)).bold(),
                Cell::from(Text::from(disp.path.as_str()).alignment(Alignment::Center)),
                Cell::from(Text::from(disp.size.as_str()).alignment(Alignment::Center)),
                Cell::from(Text::from(disp.time.as_str()).alignment(Alignment::Center)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let header_name = t!("tui.header_name");
    let header_path = t!("tui.header_path");
    let header_size = t!("tui.header_size");
    let header_time = t!("tui.header_time");
    let header_cells = vec![
        " ",
        header_name.as_ref(),
        header_path.as_ref(),
        header_size.as_ref(),
        header_time.as_ref(),
    ];
    let header_cells: Vec<Cell> = header_cells
        .iter()
        .map(|h| {
            Cell::from(
                Text::from(Span::styled(*h, Style::default().fg(Color::Cyan).bold()))
                    .alignment(Alignment::Center),
            )
        })
        .collect();
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    let widths = [
        Constraint::Length(3),
        Constraint::Max(20),
        Constraint::Fill(1),
        Constraint::Length(8),
        Constraint::Length(14),
    ];

    let filtered_str = if app.filtered.len() < app.items.len() {
        format!(" {}", t!("tui.filtered", count = app.items.len()))
    } else {
        String::new()
    };
    let plural = if app.filtered.len() == 1 { "" } else { "s" };
    let title_text = t!("tui.title", count = app.filtered.len(), plural = plural);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(30, 30, 50))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{258e}")
        .block(
            Block::default()
                .title(format!(" {}{} ", title_text, filtered_str))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_stateful_widget(table, main_area, &mut app.table_state.clone());

    let status_text = match app.mode {
        TuiMode::Normal => {
            let sel = app.visible_selected().len();
            t!("tui.status_normal", count = sel)
        }
        TuiMode::Searching => t!("tui.status_search", query = &app.search_query),
        TuiMode::Help => t!("tui.status_help"),
        TuiMode::Action => t!("tui.status_action"),
    };
    let status_bar = Paragraph::new(Line::from(Span::styled(
        status_text,
        Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 60)),
    )));
    frame.render_widget(status_bar, status_area);

    // Search input popup
    if app.mode == TuiMode::Searching {
        let popup_area = centered_rect(50, 3, area);
        let search_title = t!("tui.search_title");
        let search_block = Block::default()
            .title(search_title.as_ref())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let input = Paragraph::new(app.search_query.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(search_block);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(input, popup_area);
    }

    // Help popup
    if app.mode == TuiMode::Help {
        let popup_area = centered_rect(44, app.help_items.len() as u16 + 2, area);
        let items: Vec<Line> = app
            .help_items
            .iter()
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::styled(
                        format!(" {:<14}", key),
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                    Span::styled(desc, Style::default().fg(Color::White)),
                ])
            })
            .collect();
        let help_title = t!("tui.help_title");
        let help_block = Block::default()
            .title(help_title.as_ref())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let help_para = Paragraph::new(items).block(help_block);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(help_para, popup_area);
    }

    // Action popup
    if app.mode == TuiMode::Action {
        let popup_area = centered_rect(46, 6, area);
        let cursor = app.action_cursor;

        let btn_style = |idx: usize| {
            if idx == cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            }
        };
        let arrow = |idx: usize| -> &str { if idx == cursor { " > " } else { "   " } };

        let actions = ActionChoice::all();
        let lines = vec![
            Line::from(Span::styled(
                format!("{}{}", arrow(0), actions[0].label()),
                btn_style(0),
            )),
            Line::from(Span::styled(
                format!("{}{}", arrow(1), actions[1].label()),
                btn_style(1),
            )),
            Line::from(vec![
                Span::styled(
                    format!("{}{}", arrow(2), actions[2].label()),
                    btn_style(2),
                ),
                Span::raw("      "),
                Span::styled(
                    format!("{}{}", arrow(3), actions[3].label()),
                    btn_style(3),
                ),
            ]),
        ];

        let action_title = t!("tui.action_title");
        let action_block = Block::default()
            .title(action_title.as_ref())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let action_para = Paragraph::new(lines)
            .block(action_block)
            .alignment(Alignment::Center);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(action_para, popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Length((r.height.saturating_sub(percent_y)) / 2),
        Constraint::Length(percent_y),
        Constraint::Min(0),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
        Constraint::Length(percent_x),
        Constraint::Min(0),
    ])
    .split(popup_layout[1])[1]
}

/// Interactive TUI trash manager.
pub fn select_visible(conn: &mut Connection) -> Result<Vec<Trash>> {
    let mut stmt = conn.prepare("SELECT id, time, path, hash, size FROM trash")?;
    let items: Vec<Trash> = stmt
        .query_map([], |row| {
            Ok(Trash {
                id: row.get(0)?,
                time: row.get(1)?,
                path: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    if items.is_empty() {
        return Ok(Vec::new());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let result = tui_run(terminal, items, conn);
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn tui_run(
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    items: Vec<Trash>,
    conn: &mut Connection,
) -> Result<Vec<Trash>> {
    let mut t = terminal;
    let mut app = TuiApp::new(items)?;
    loop {
        t.draw(|f| tui_render(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.mode {
                TuiMode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Vec::new()),
                    KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
                    KeyCode::Char('J') => app.select_next_n(5),
                    KeyCode::Char('K') => app.select_prev_n(5),
                    KeyCode::PageDown => app.page_down(),
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::Home => app.go_top(),
                    KeyCode::End => app.go_bottom(),
                    KeyCode::Char(' ') => app.toggle_selection(),
                    KeyCode::Char('a') => app.select_all(),
                    KeyCode::Char('A') => app.invert_selection(),
                    KeyCode::Char('/') => {
                        app.mode = TuiMode::Searching;
                        app.search_query.clear();
                    }
                    KeyCode::Char('?') | KeyCode::Char('h') => {
                        app.mode = TuiMode::Help;
                    }
                    KeyCode::Enter => {
                        app.action_cursor = 0;
                        app.mode = TuiMode::Action;
                    }
                    _ => {}
                },

                TuiMode::Searching => match key.code {
                    KeyCode::Esc => {
                        app.mode = TuiMode::Normal;
                        app.search_query.clear();
                        app.apply_search();
                    }
                    KeyCode::Enter => {
                        app.mode = TuiMode::Normal;
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.apply_search();
                    }
                    KeyCode::Char(c) => {
                        if c != '/' {
                            app.search_query.push(c);
                            app.apply_search();
                        }
                    }
                    _ => {}
                },

                TuiMode::Help => {
                    app.mode = TuiMode::Normal;
                }

                TuiMode::Action => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.mode = TuiMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.action_cursor = match app.action_cursor {
                            0 => 0,
                            1 => 0,
                            2 => 1,
                            3 => 1,
                            _ => app.action_cursor.saturating_sub(1),
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.action_cursor = match app.action_cursor {
                            0 => 1,
                            1 => 2,
                            _ => 3,
                        };
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if app.action_cursor == 3 {
                            app.action_cursor = 2;
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if app.action_cursor == 2 {
                            app.action_cursor = 3;
                        }
                    }
                    KeyCode::Enter => {
                        let choice = &ActionChoice::all()[app.action_cursor];
                        match choice {
                            ActionChoice::Delete => {
                                let sel = app.visible_selected();
                                if sel.is_empty() {
                                    return Ok(Vec::new());
                                }
                                for &orig_idx in &sel {
                                    let id = app.items[orig_idx].0.id;
                                    conn.execute("DELETE FROM trash WHERE id = ?1", params![id])?;
                                }
                                return Ok(Vec::new());
                            }
                            ActionChoice::Restore => {
                                let sel = app.visible_selected();
                                if sel.is_empty() {
                                    return Ok(Vec::new());
                                }
                                let mut result = Vec::new();
                                for &orig_idx in &sel {
                                    let trash = app.items[orig_idx].0.clone();
                                    let id = trash.id;
                                    conn.execute("DELETE FROM trash WHERE id = ?1", params![id])?;
                                    result.push(trash);
                                }
                                return Ok(result);
                            }
                            ActionChoice::Exit => {
                                return Ok(Vec::new());
                            }
                            ActionChoice::Cancel => {
                                app.mode = TuiMode::Normal;
                            }
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}
