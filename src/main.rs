mod board;
mod sys;

use board::{Agent, Board, Column, Issue, Label, build, owner_line};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
            MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
    },
    layout::{Alignment, Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

const DOUBLE_CLICK: Duration = Duration::from_millis(400);

enum Req {
    Refresh,
    Agents,
    Move(Issue, Column),
}

enum Resp {
    Issues(Result<Vec<Issue>, String>),
    Agents(Result<Vec<Agent>, String>),
    Moved(Result<(), String>),
}

fn worker(ctx: sys::Ctx, rx: Receiver<Req>, tx: Sender<Resp>) {
    let refresh = |tx: &Sender<Resp>| {
        let _ = tx.send(Resp::Issues(sys::fetch_issues(&ctx.repo)));
        let _ = tx.send(Resp::Agents(sys::fetch_agents(&ctx.repo_key)));
    };
    for req in rx {
        match req {
            Req::Refresh => refresh(&tx),
            Req::Agents => {
                let _ = tx.send(Resp::Agents(sys::fetch_agents(&ctx.repo_key)));
            }
            Req::Move(issue, to) => {
                let _ = tx.send(Resp::Moved(sys::move_issue(&ctx.repo, &issue, to)));
                refresh(&tx);
            }
        }
    }
}

struct App {
    repo: String,
    issues: Vec<Issue>,
    agents: Vec<Agent>,
    board: Board,
    col: usize,
    row: usize,
    /// [issues, agents, move]
    errs: [Option<String>; 3],
    /// Transient message from a key action; cleared on next key.
    note: Option<String>,
    loaded: bool,
    refreshed: Option<Instant>,
    tx: Sender<Req>,
    /// Per-column list state; keeps the scroll offset the mouse hit-test needs.
    states: [ListState; 5],
    /// Column rects from the last draw.
    areas: Vec<Rect>,
    /// Card under the mouse button, for drag-to-move.
    drag_from: Option<usize>,
    last_click: Option<(usize, usize, Instant)>,
    /// Last mouse cell; hover is recomputed from it every frame so it survives scroll/refresh.
    mouse_pos: Option<(u16, u16)>,
}

impl App {
    fn ncols(&self) -> usize {
        if self.board.unlinked.is_empty() { 4 } else { 5 }
    }

    fn col_len(&self, col: usize) -> usize {
        if col < 4 { self.board.columns[col].len() } else { self.board.unlinked.len() }
    }

    fn rebuild(&mut self) {
        self.board = build(self.issues.clone(), self.agents.clone());
        self.col = self.col.min(self.ncols() - 1);
        self.row = self.row.min(self.col_len(self.col).saturating_sub(1));
    }

    fn apply(&mut self, resp: Resp) {
        match resp {
            Resp::Issues(r) => {
                self.errs[0] = r.as_ref().err().cloned();
                if let Ok(v) = r {
                    self.issues = v;
                    self.loaded = true;
                    self.refreshed = Some(Instant::now());
                    self.rebuild();
                }
            }
            Resp::Agents(r) => {
                self.errs[1] = r.as_ref().err().cloned();
                if let Ok(v) = r {
                    self.agents = v;
                    self.rebuild();
                }
            }
            Resp::Moved(r) => self.errs[2] = r.err(),
        }
    }

    fn move_selected(&mut self, delta: isize) {
        let to = self.col as isize + delta;
        if (0..4).contains(&to) {
            self.move_to(to as usize);
        }
    }

    /// Moves the selected card to column `to` (optimistic; GitHub is written by the worker).
    fn move_to(&mut self, to: usize) {
        if self.col >= 4 || to >= 4 || to == self.col {
            return;
        }
        let to = Column::ALL[to];
        let Some(card) = self.board.columns[self.col].get(self.row) else { return };
        let orig = card.issue.clone();
        let n = orig.number;
        if let Some(i) = self.issues.iter_mut().find(|i| i.number == n) {
            i.labels.retain(|l| !l.name.starts_with("status:"));
            match to.label() {
                Some(l) => {
                    i.state = "OPEN".into();
                    i.labels.push(Label { name: l.into() });
                }
                None => i.state = "CLOSED".into(),
            }
        }
        self.rebuild();
        self.col = to as usize;
        self.row = self.board.columns[self.col].iter().position(|c| c.issue.number == n).unwrap_or(0);
        let _ = self.tx.send(Req::Move(orig, to));
    }

    fn selected_agent(&self) -> Option<&Agent> {
        if self.col >= 4 {
            self.board.unlinked.get(self.row)
        } else {
            self.board.columns[self.col].get(self.row)?.agents.first()
        }
    }

    /// Focuses the selected card's agent. Returns true when the app should exit.
    fn focus(&mut self) -> bool {
        match self.selected_agent() {
            None => self.note = Some("no agent linked".into()),
            Some(a) => match sys::focus_agent(&a.pane_id) {
                Ok(()) => return true,
                Err(e) => self.note = Some(e),
            },
        }
        false
    }

    /// Returns true when the app should exit.
    fn key(&mut self, code: KeyCode) -> bool {
        self.note = None;
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('h') | KeyCode::Left => {
                self.col = self.col.saturating_sub(1);
                self.row = self.row.min(self.col_len(self.col).saturating_sub(1));
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.col = (self.col + 1).min(self.ncols() - 1);
                self.row = self.row.min(self.col_len(self.col).saturating_sub(1));
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.row = (self.row + 1).min(self.col_len(self.col).saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => self.row = self.row.saturating_sub(1),
            KeyCode::Char('H') => self.move_selected(-1),
            KeyCode::Char('L') => self.move_selected(1),
            KeyCode::Char('r') => {
                let _ = self.tx.send(Req::Refresh);
            }
            KeyCode::Char('o') => {
                if self.col < 4 {
                    if let Some(c) = self.board.columns[self.col].get(self.row) {
                        if let Err(e) = sys::open_url(&c.issue.url) {
                            self.note = Some(e);
                        }
                    }
                }
            }
            KeyCode::Enter => return self.focus(),
            _ => {}
        }
        false
    }

    /// Column under the cell and the card row there (None on borders/empty space).
    fn hit(&self, x: u16, y: u16) -> Option<(usize, Option<usize>)> {
        let col = self.areas.iter().position(|a| a.contains(Position::new(x, y)))?;
        let a = self.areas[col];
        let row = y
            .checked_sub(a.y + 1)
            .filter(|_| y + 1 < a.y + a.height)
            // cards are 2 lines tall
            .map(|d| self.states[col].offset() + usize::from(d) / 2)
            .filter(|r| *r < self.col_len(col));
        Some((col, row))
    }

    /// Returns true when the app should exit.
    fn mouse(&mut self, m: MouseEvent) -> bool {
        let hit = self.hit(m.column, m.row);
        self.mouse_pos = Some((m.column, m.row));
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.note = None;
                let Some((col, row)) = hit else { return false };
                self.col = col;
                self.drag_from = row;
                let Some(row) = row else {
                    self.row = self.row.min(self.col_len(col).saturating_sub(1));
                    return false;
                };
                self.row = row;
                let double = self
                    .last_click
                    .is_some_and(|(c, r, t)| (c, r) == (col, row) && t.elapsed() < DOUBLE_CLICK);
                if double {
                    self.last_click = None;
                    return self.focus();
                }
                self.last_click = Some((col, row, Instant::now()));
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // Card was picked on Down (self.col/self.row); dropping on another column moves it.
                if self.drag_from.take().is_some() {
                    if let Some((to, _)) = hit {
                        self.move_to(to);
                    }
                }
            }
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let Some((col, _)) = hit else { return false };
                if col != self.col {
                    self.col = col;
                    self.row = self.row.min(self.col_len(col).saturating_sub(1));
                }
                if m.kind == MouseEventKind::ScrollDown {
                    self.row = (self.row + 1).min(self.col_len(col).saturating_sub(1));
                } else {
                    self.row = self.row.saturating_sub(1);
                }
            }
            _ => {}
        }
        false
    }

    fn status(&self) -> String {
        if let Some(n) = &self.note {
            return n.clone();
        }
        if let Some(e) = self.errs.iter().flatten().next() {
            return e.clone();
        }
        match self.refreshed {
            Some(t) if self.loaded => format!("{} · refreshed {}s ago", self.repo, t.elapsed().as_secs()),
            _ => "loading…".into(),
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let [main, foot] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(f.area());
        let n = self.ncols();
        let areas = Layout::horizontal(vec![Constraint::Fill(1); n]).split(main);
        let hover = self
            .mouse_pos
            .and_then(|(x, y)| self.hit(x, y))
            .and_then(|(c, r)| Some((c, r?)));
        self.areas = areas.to_vec();
        for (i, area) in areas.iter().enumerate() {
            let (title, items): (String, Vec<ListItem>) = if i < 4 {
                let cards = &self.board.columns[i];
                let items = cards
                    .iter()
                    .map(|c| {
                        let style = match c.agents.first().map(|a| a.agent_status.as_str()) {
                            Some("working") => Style::new().fg(Color::Yellow),
                            Some("blocked") => Style::new().fg(Color::Red),
                            Some("done") => Style::new().fg(Color::Green),
                            Some(_) => Style::new().fg(Color::Gray),
                            None => Style::new().add_modifier(Modifier::DIM),
                        };
                        ListItem::new(Text::from(vec![
                            Line::raw(format!("#{} {}", c.issue.number, c.issue.title)),
                            Line::styled(owner_line(c), style),
                        ]))
                    })
                    .collect();
                (format!("{} ({})", Column::ALL[i].title(), cards.len()), items)
            } else {
                let items = self
                    .board
                    .unlinked
                    .iter()
                    .map(|a| {
                        ListItem::new(Text::from(vec![
                            Line::raw(format!("🤖 {} {} {}", a.agent, a.pane_id, a.agent_status)),
                            Line::styled(a.terminal_title_stripped.clone(), Style::new().add_modifier(Modifier::DIM)),
                        ]))
                    })
                    .collect();
                (format!("Agents (no issue) ({})", self.board.unlinked.len()), items)
            };
            let items: Vec<ListItem> = items
                .into_iter()
                .enumerate()
                .map(|(r, it)| {
                    let hovered = hover == Some((i, r)) && (i, r) != (self.col, self.row);
                    if hovered { it.style(Style::new().bg(Color::Indexed(238))) } else { it }
                })
                .collect();
            let list = List::new(items)
                .block(Block::bordered().title(title))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
            let sel = (i == self.col && self.col_len(i) > 0).then_some(self.row);
            self.states[i].select(sel);
            f.render_stateful_widget(list, *area, &mut self.states[i]);
        }
        let help = "←→/hl col  ↑↓/jk card  H/L move  ⏎ focus agent  o open  r refresh  q quit  mouse: click · drag=move · 2×click=focus";
        f.render_widget(Paragraph::new(format!("{help} | {}", self.status())), foot);
    }
}

fn error_screen(term: &mut DefaultTerminal, msg: &str) -> std::io::Result<()> {
    loop {
        term.draw(|f| {
            let [_, mid, _] = Layout::vertical([Constraint::Fill(1), Constraint::Length(6), Constraint::Fill(1)])
                .areas(f.area());
            f.render_widget(
                Paragraph::new(format!("{msg}\n\nq to quit"))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                mid,
            );
        })?;
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Press && matches!(k.code, KeyCode::Char('q') | KeyCode::Esc) {
                return Ok(());
            }
        }
    }
}

fn run(term: &mut DefaultTerminal, ctx: sys::Ctx) -> std::io::Result<()> {
    let (req_tx, req_rx) = mpsc::channel();
    let (resp_tx, resp_rx) = mpsc::channel();
    let repo = ctx.repo.clone();
    std::thread::spawn(move || worker(ctx, req_rx, resp_tx));
    let _ = req_tx.send(Req::Refresh);
    let mut app = App {
        repo,
        issues: vec![],
        agents: vec![],
        board: build(vec![], vec![]),
        col: 0,
        row: 0,
        errs: [None, None, None],
        note: None,
        loaded: false,
        refreshed: None,
        tx: req_tx,
        states: std::array::from_fn(|_| ListState::default()),
        areas: vec![],
        drag_from: None,
        last_click: None,
        mouse_pos: None,
    };
    let (mut last_full, mut last_agents) = (Instant::now(), Instant::now());
    loop {
        while let Ok(r) = resp_rx.try_recv() {
            app.apply(r);
        }
        term.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press && app.key(k.code) => return Ok(()),
                Event::Mouse(m) if app.mouse(m) => return Ok(()),
                _ => {}
            }
        }
        if last_full.elapsed() >= Duration::from_secs(30) {
            let _ = app.tx.send(Req::Refresh);
            last_full = Instant::now();
            last_agents = last_full;
        } else if last_agents.elapsed() >= Duration::from_secs(3) {
            let _ = app.tx.send(Req::Agents);
            last_agents = Instant::now();
        }
    }
}

fn main() -> std::io::Result<()> {
    let ctx = sys::resolve_ctx();
    let mut term = ratatui::init();
    let _ = execute!(std::io::stdout(), EnableMouseCapture);
    let res = match ctx {
        Ok(ctx) => run(&mut term, ctx),
        Err(e) => error_screen(&mut term, &e),
    };
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    res
}
