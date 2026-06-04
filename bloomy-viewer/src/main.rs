use std::env;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Seek;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use bloomy::storage::wal;
use bloomy::storage::wal::ReadRecord;
use bloomy::storage::wal::WalRecord;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

const POLL_INTERVAL: Duration = Duration::from_millis(250);

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let args = Args::parse(env::args().skip(1))?;
    let mut app = App::new(args.path, args.tail);
    app.refresh();
    run_terminal(&mut app)
}

#[derive(Debug)]
struct Args {
    path: PathBuf,
    tail: bool,
}

impl Args {
    fn parse(args: impl Iterator<Item = String>) -> io::Result<Self> {
        let mut path = None;
        let mut tail = true;

        for arg in args {
            match arg.as_str() {
                "--snapshot" => tail = false,
                "--tail" => tail = true,
                "-h" | "--help" => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        usage().to_string(),
                    ));
                }
                _ if arg.starts_with('-') => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument: {arg}\n\n{}", usage()),
                    ));
                }
                _ => {
                    if path.replace(PathBuf::from(arg)).is_some() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("expected one WAL path\n\n{}", usage()),
                        ));
                    }
                }
            }
        }

        let path = path.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("missing WAL path\n\n{}", usage()),
            )
        })?;

        Ok(Self { path, tail })
    }
}

fn usage() -> &'static str {
    "usage: bloomy-viewer [--tail|--snapshot] path/to/bloomy.wal"
}

#[derive(Debug)]
struct App {
    path: PathBuf,
    records: Vec<ViewedRecord>,
    selected: usize,
    tail: bool,
    status: ViewerStatus,
    last_refresh: Instant,
}

impl App {
    fn new(path: PathBuf, tail: bool) -> Self {
        Self {
            path,
            records: Vec::new(),
            selected: 0,
            tail,
            status: ViewerStatus::Waiting("opening WAL".to_string()),
            last_refresh: Instant::now(),
        }
    }

    fn refresh(&mut self) {
        match inspect_wal(&self.path) {
            Ok(snapshot) => {
                self.records = snapshot.records;
                self.status = snapshot.status;

                if self.records.is_empty() {
                    self.selected = 0;
                } else if self.selected >= self.records.len() {
                    self.selected = self.records.len() - 1;
                }
            }
            Err(error) => {
                self.status = ViewerStatus::Corrupt(error.to_string());
            }
        }

        self.last_refresh = Instant::now();
    }

    fn next(&mut self) {
        if self.records.is_empty() {
            return;
        }

        self.selected = (self.selected + 1).min(self.records.len() - 1);
    }

    fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn selected_record(&self) -> Option<&ViewedRecord> {
        self.records.get(self.selected)
    }
}

#[derive(Debug)]
struct WalSnapshot {
    records: Vec<ViewedRecord>,
    status: ViewerStatus,
}

#[derive(Debug)]
enum ViewerStatus {
    CleanEof { offset: u64 },
    PartialTail { offset: u64 },
    Waiting(String),
    Corrupt(String),
}

#[derive(Debug)]
struct ViewedRecord {
    index: usize,
    offset: u64,
    end_offset: u64,
    kind: RecordKind,
    key: Vec<u8>,
    value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
enum RecordKind {
    Put,
    Delete,
}

fn inspect_wal(path: &Path) -> io::Result<WalSnapshot> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(WalSnapshot {
                records: Vec::new(),
                status: ViewerStatus::Waiting("file does not exist yet".to_string()),
            });
        }
        Err(error) => return Err(error),
    };

    if metadata.len() < wal::FILE_HEADER_BYTES as u64 {
        return Ok(WalSnapshot {
            records: Vec::new(),
            status: ViewerStatus::Waiting(format!(
                "waiting for WAL header: {} / {} bytes",
                metadata.len(),
                wal::FILE_HEADER_BYTES
            )),
        });
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    wal::read_header(&mut reader).map_err(as_io_error)?;

    let mut records = Vec::new();

    loop {
        let offset = reader.stream_position()?;

        match wal::read_record(&mut reader).map_err(as_io_error)? {
            ReadRecord::Record(record) => {
                let end_offset = reader.stream_position()?;
                records.push(ViewedRecord::from_record(
                    records.len(),
                    offset,
                    end_offset,
                    record,
                ));
            }
            ReadRecord::CleanEof => {
                return Ok(WalSnapshot {
                    records,
                    status: ViewerStatus::CleanEof { offset },
                });
            }
            ReadRecord::PartialTail => {
                return Ok(WalSnapshot {
                    records,
                    status: ViewerStatus::PartialTail { offset },
                });
            }
        }
    }
}

impl ViewedRecord {
    fn from_record(index: usize, offset: u64, end_offset: u64, record: WalRecord) -> Self {
        match record {
            WalRecord::Put { key, value } => Self {
                index,
                offset,
                end_offset,
                kind: RecordKind::Put,
                key,
                value: Some(value),
            },
            WalRecord::Delete { key } => Self {
                index,
                offset,
                end_offset,
                kind: RecordKind::Delete,
                key,
                value: None,
            },
        }
    }
}

fn as_io_error(error: bloomy::error::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn run_terminal(app: &mut App) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = event_loop(&mut terminal, app);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;

        if app.tail && app.last_refresh.elapsed() >= POLL_INTERVAL {
            app.refresh();
        }

        if event::poll(POLL_INTERVAL)?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('r') => app.refresh(),
                KeyCode::Char('t') => {
                    app.tail = !app.tail;
                    app.refresh();
                }
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                KeyCode::Home => app.selected = 0,
                KeyCode::End if !app.records.is_empty() => {
                    app.selected = app.records.len() - 1;
                }
                _ => {}
            }
        }
    }
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(page[1]);

    draw_header(frame, page[0], app);
    draw_records(frame, body[0], app);
    draw_details(frame, body[1], app);
    draw_status(frame, page[2], app);
}

fn draw_header(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let title = Line::from(vec![
        Span::styled("Bloomy WAL Viewer", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(
            app.path.display().to_string(),
            Style::default().fg(Color::Gray),
        ),
    ]);
    let mode = if app.tail { "tailing" } else { "snapshot" };
    let header = Paragraph::new(vec![title, Line::from(format!("mode: {mode}"))])
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

fn draw_records(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let items = app
        .records
        .iter()
        .map(|record| {
            let kind = match record.kind {
                RecordKind::Put => "PUT   ",
                RecordKind::Delete => "DELETE",
            };
            let line = format!(
                "#{:<4} {:>10}..{:<10} {} key={}",
                record.index,
                record.offset,
                record.end_offset,
                kind,
                preview_key(&record.key, 36),
            );

            ListItem::new(line)
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    if !app.records.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(Block::default().title("records").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_details(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let lines = match app.selected_record() {
        Some(record) => detail_lines(record),
        None => vec![
            Line::from("No records decoded yet."),
            Line::from("Open a WAL file or wait for the writer to append records."),
        ],
    };

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .title("selected record")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(details, area);
}

fn detail_lines(record: &ViewedRecord) -> Vec<Line<'static>> {
    let kind = match record.kind {
        RecordKind::Put => "PUT",
        RecordKind::Delete => "DELETE",
    };
    let mut lines = vec![
        Line::from(format!("index       {}", record.index)),
        Line::from(format!("offset      {}", record.offset)),
        Line::from(format!("end offset  {}", record.end_offset)),
        Line::from(format!("bytes       {}", record.end_offset - record.offset)),
        Line::from(format!("kind        {kind}")),
        Line::from(format!("key len     {}", record.key.len())),
    ];

    match &record.value {
        Some(value) => {
            lines.push(Line::from(format!("value len   {}", value.len())));
            lines.push(Line::from(format!(
                "value       {}",
                preview_value(value, 96)
            )));
        }
        None => {
            lines.push(Line::from("value len   0"));
            lines.push(Line::from("value       tombstone"));
        }
    }

    lines
}

fn draw_status(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let (label, color) = match &app.status {
        ViewerStatus::CleanEof { offset } => {
            (format!("clean EOF at offset {offset}"), Color::Green)
        }
        ViewerStatus::PartialTail { offset } => {
            (format!("partial record at offset {offset}"), Color::Yellow)
        }
        ViewerStatus::Waiting(message) => (message.clone(), Color::Yellow),
        ViewerStatus::Corrupt(message) => (format!("corrupt WAL: {message}"), Color::Red),
    };
    let controls = "q quit | j/k move | r refresh | t toggle tail | home/end";
    let status = Paragraph::new(vec![
        Line::from(vec![Span::styled(label, Style::default().fg(color))]),
        Line::from(controls),
    ])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status, area);
}

fn preview_value(bytes: &[u8], max: usize) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => truncate_chars(text, max),
        Err(_) => format!("<binary {} bytes>", bytes.len()),
    }
}

fn preview_key(bytes: &[u8], max: usize) -> String {
    if let Ok(text) = std::str::from_utf8(bytes)
        && text.chars().all(|character| !character.is_control())
    {
        return truncate_chars(text, max);
    }

    format!("<binary {} bytes>", bytes.len())
}

fn truncate_chars(text: &str, max: usize) -> String {
    let mut truncated = String::new();

    for (index, character) in text.chars().enumerate() {
        if index == max {
            truncated.push_str("...");
            break;
        }

        truncated.push(character);
    }

    truncated
}
