mod ui;

use crate::protocol::{App, Project, Request, Response};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use rkyv::util::AlignedVec;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;

/// Item in the suggestion list — either an App or a Project.
#[derive(Clone)]
pub enum SuggestionItem {
    App(App),
    Project(Project),
}

impl SuggestionItem {
    pub fn name(&self) -> &str {
        match self {
            SuggestionItem::App(a) => &a.name,
            SuggestionItem::Project(p) => &p.name,
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            SuggestionItem::App(_) => "App",
            SuggestionItem::Project(_) => "Project",
        }
    }
}

pub struct PopupState {
    pub query: String,
    pub all_items: Vec<SuggestionItem>,
    pub filtered: Vec<SuggestionItem>,
    pub selected: usize,
    pub should_quit: bool,
}

impl PopupState {
    pub fn new(projects: Vec<Project>, apps: Vec<App>) -> Self {
        let mut all_items: Vec<SuggestionItem> = Vec::with_capacity(projects.len() + apps.len());

        for p in projects {
            all_items.push(SuggestionItem::Project(p));
        }
        for a in apps {
            all_items.push(SuggestionItem::App(a));
        }

        let filtered = all_items.clone();

        PopupState {
            query: String::new(),
            all_items,
            filtered,
            selected: 0,
            should_quit: false,
        }
    }

    pub fn update_filter(&mut self) {
        let q = self.query.to_lowercase();

        if q.is_empty() {
            self.filtered = self.all_items.clone();
        } else {
            self.filtered = self
                .all_items
                .iter()
                .filter(|item| item.name().to_lowercase().contains(&q))
                .cloned()
                .collect();
        }

        // Keep selected in bounds
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }
}

/// Connect to the daemon and fetch all projects and apps.
fn fetch_data() -> Result<(Vec<Project>, Vec<App>), String> {
    let mut stream =
        UnixStream::connect("/tmp/aooff.sock").map_err(|e| format!("Connect failed: {}", e))?;

    // Serialize and send the GetAll request
    let request_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&Request::GetAll)
        .map_err(|e| format!("Serialize failed: {}", e))?;

    stream
        .write_all(&request_bytes)
        .map_err(|e| format!("Write failed: {}", e))?;

    // Shutdown write side so the server gets EOF
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| format!("Shutdown failed: {}", e))?;

    // Read response
    let mut buffer = Vec::new();
    stream
        .read_to_end(&mut buffer)
        .map_err(|e| format!("Read failed: {}", e))?;

    let mut aligned: AlignedVec<16> = AlignedVec::new();
    aligned.extend_from_slice(&buffer);

    let response = rkyv::from_bytes::<Response, rkyv::rancor::Error>(&aligned)
        .map_err(|e| format!("Deserialize failed: {}", e))?;

    match response {
        Response::All { projects, apps } => Ok((projects, apps)),
        Response::Projects(projects) => Ok((projects, Vec::new())),
        Response::Error(e) => Err(e),
    }
}

pub fn run() {
    // Fetch data from daemon
    let start = std::time::Instant::now();
    let (projects, apps) = match fetch_data() {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            eprintln!("Make sure the daemon is running (aooff)");
            std::process::exit(1);
        }
    };
    let elapsed = start.elapsed();
    // Write timing to file (stderr is taken over by the TUI)
    let _ = std::fs::write(
        "/tmp/aooff_popup_timing.txt",
        format!(
            "IPC fetch: {} projects + {} apps in {:.2?}",
            projects.len(),
            apps.len(),
            elapsed
        ),
    );

    let mut state = PopupState::new(projects, apps);

    // Setup terminal
    terminal::enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .expect("Failed to enter alternate screen");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    // Event loop
    loop {
        terminal
            .draw(|f| ui::draw(f, &state))
            .expect("Failed to draw");

        if event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Esc => {
                        state.should_quit = true;
                    }
                    KeyCode::Backspace => {
                        state.query.pop();
                        state.update_filter();
                    }
                    KeyCode::Up => {
                        if state.selected > 0 {
                            state.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if !state.filtered.is_empty()
                            && state.selected < state.filtered.len() - 1
                        {
                            state.selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // For now, just quit — action handling can be added later
                        state.should_quit = true;
                    }
                    KeyCode::Char(c) => {
                        state.query.push(c);
                        state.update_filter();
                    }
                    _ => {}
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    // Restore terminal
    terminal::disable_raw_mode().expect("Failed to disable raw mode");
    io::stdout()
        .execute(LeaveAlternateScreen)
        .expect("Failed to leave alternate screen");
}
