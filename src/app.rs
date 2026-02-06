use std::io::stdout;
use std::panic;

use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use tokio::sync::mpsc;

use crate::{http, ui};

#[derive(Debug, Clone, Default)]
pub enum ResponseStatus {
    #[default]
    Empty,
    Loading,
    Success(ResponseData),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub const ALL: [HttpMethod; 5] = [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            HttpMethod::Get => 0,
            HttpMethod::Post => 1,
            HttpMethod::Put => 2,
            HttpMethod::Patch => 3,
            HttpMethod::Delete => 4,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestState {
    pub url: String,
    pub method: HttpMethod,
    pub headers: String,
    pub body: String,
    pub url_cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Panel {
    Sidebar,
    #[default]
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestField {
    Method,
    #[default]
    Url,
    Headers,
    Body,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FocusState {
    pub panel: Panel,
    pub request_field: RequestField,
}

pub struct App {
    running: bool,
    pub request: RequestState,
    pub focus: FocusState,
    pub response: ResponseStatus,
    pub client: Client,
    pub input_mode: InputMode,
    pub response_scroll: u16,
    pub loading_tick: u8,
    pub show_help: bool,
    pub show_method_popup: bool,
    pub method_popup_index: usize,
    pub sidebar_visible: bool,
}

impl App {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            running: true,
            request: RequestState::default(),
            focus: FocusState::default(),
            response: ResponseStatus::Empty,
            client,
            input_mode: InputMode::Normal,
            response_scroll: 0,
            loading_tick: 0,
            show_help: false,
            show_method_popup: false,
            method_popup_index: 0,
            sidebar_visible: true,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.install_panic_hook();
        self.setup_terminal()?;

        let result = self.event_loop().await;

        self.restore_terminal()?;
        result
    }

    fn install_panic_hook(&self) {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = stdout().execute(LeaveAlternateScreen);
            original_hook(panic_info);
        }));
    }

    fn setup_terminal(&self) -> Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn restore_terminal(&self) -> Result<()> {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    async fn event_loop(&mut self) -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let (tx, mut rx) = mpsc::channel::<Result<ResponseData, String>>(1);

        while self.running {
            terminal.draw(|frame| {
                ui::render(frame, self);
            })?;

            if let Ok(result) = rx.try_recv() {
                self.response = match result {
                    Ok(data) => ResponseStatus::Success(data),
                    Err(e) => ResponseStatus::Error(e),
                };
                self.response_scroll = 0;
            }

            if matches!(self.response, ResponseStatus::Loading) {
                self.loading_tick = self.loading_tick.wrapping_add(1);
            }

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key, tx.clone());
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent, tx: mpsc::Sender<Result<ResponseData, String>>) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key, tx),
            InputMode::Insert => self.handle_insert_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent, tx: mpsc::Sender<Result<ResponseData, String>>) {
        // Handle method popup navigation when open
        if self.show_method_popup {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.method_popup_index = (self.method_popup_index + 1) % HttpMethod::ALL.len();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.method_popup_index = if self.method_popup_index == 0 {
                        HttpMethod::ALL.len() - 1
                    } else {
                        self.method_popup_index - 1
                    };
                }
                KeyCode::Enter => {
                    self.request.method = HttpMethod::from_index(self.method_popup_index);
                    self.show_method_popup = false;
                }
                KeyCode::Esc => {
                    self.show_method_popup = false;
                }
                _ => {}
            }
            return;
        }

        let in_request_panel = self.focus.panel == Panel::Request;
        let in_response_panel = self.focus.panel == Panel::Response;
        let in_method_field = self.focus.request_field == RequestField::Method;

        // Horizontal navigation with h/l between Method and URL fields
        if in_request_panel {
            let in_url_field = self.focus.request_field == RequestField::Url;
            match key.code {
                KeyCode::Left | KeyCode::Char('h') if in_url_field => {
                    self.focus.request_field = RequestField::Method;
                    return;
                }
                KeyCode::Right | KeyCode::Char('l') if in_method_field => {
                    self.focus.request_field = RequestField::Url;
                    return;
                }
                _ => {}
            }
        }

        if in_request_panel && in_method_field {
            if let KeyCode::Enter = key.code {
                // Open method popup
                self.method_popup_index = self.request.method.index();
                self.show_method_popup = true;
                return;
            }
        }

        if in_response_panel {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.response_scroll = self.response_scroll.saturating_sub(1);
                    return;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.response_scroll = self.response_scroll.saturating_add(1);
                    return;
                }
                _ => {}
            }
        }

        // Ctrl+E toggles sidebar
        if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.sidebar_visible = !self.sidebar_visible;
            return;
        }

        match key.code {
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('i') => {
                if in_request_panel && self.is_editable_field() {
                    self.input_mode = InputMode::Insert;
                    let _ = stdout().execute(SetCursorStyle::SteadyUnderScore);
                }
            }
            KeyCode::Enter => {
                if in_request_panel {
                    self.send_request(tx);
                }
            }
            KeyCode::Tab => self.cycle_panel(),
            KeyCode::Up | KeyCode::Char('k') => self.prev_field(),
            KeyCode::Down | KeyCode::Char('j') => self.next_field(),
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            _ => {}
        }
    }

    fn handle_insert_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                let _ = stdout().execute(SetCursorStyle::DefaultUserShape);
            }
            KeyCode::Enter => {
                match self.focus.request_field {
                    RequestField::Url => {
                        self.input_mode = InputMode::Normal;
                        let _ = stdout().execute(SetCursorStyle::DefaultUserShape);
                    }
                    RequestField::Headers | RequestField::Body => {
                        self.insert_char('\n');
                    }
                    RequestField::Method => {}
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Backspace => {
                self.delete_char();
            }
            _ => {}
        }
    }

    fn send_request(&mut self, tx: mpsc::Sender<Result<ResponseData, String>>) {
        if self.request.url.is_empty() {
            self.response = ResponseStatus::Error("URL is required".to_string());
            return;
        }

        if matches!(self.response, ResponseStatus::Loading) {
            return;
        }

        self.response = ResponseStatus::Loading;

        let client = self.client.clone();
        let method = self.request.method;
        let url = self.request.url.clone();
        let headers = self.request.headers.clone();
        let body = self.request.body.clone();

        tokio::spawn(async move {
            let result = http::send_request(&client, method, &url, &headers, &body).await;
            let _ = tx.send(result).await;
        });
    }

    fn is_editable_field(&self) -> bool {
        matches!(
            self.focus.request_field,
            RequestField::Url | RequestField::Headers | RequestField::Body
        )
    }

    fn insert_char(&mut self, c: char) {
        match self.focus.request_field {
            RequestField::Url => {
                self.request.url.insert(self.request.url_cursor, c);
                self.request.url_cursor += 1;
            }
            RequestField::Headers => {
                self.request.headers.push(c);
            }
            RequestField::Body => {
                self.request.body.push(c);
            }
            RequestField::Method => {}
        }
    }

    fn delete_char(&mut self) {
        match self.focus.request_field {
            RequestField::Url => {
                if self.request.url_cursor > 0 {
                    self.request.url_cursor -= 1;
                    self.request.url.remove(self.request.url_cursor);
                }
            }
            RequestField::Headers => {
                self.request.headers.pop();
            }
            RequestField::Body => {
                self.request.body.pop();
            }
            RequestField::Method => {}
        }
    }

    fn cycle_panel(&mut self) {
        // Tab cycles: Request -> Response -> Request (Sidebar skipped for now)
        self.focus.panel = match self.focus.panel {
            Panel::Sidebar => Panel::Request,
            Panel::Request => Panel::Response,
            Panel::Response => Panel::Request,
        };
    }

    fn next_field(&mut self) {
        if self.focus.panel != Panel::Request {
            return;
        }
        self.focus.request_field = match self.focus.request_field {
            RequestField::Method => RequestField::Url,
            RequestField::Url => RequestField::Headers,
            RequestField::Headers => RequestField::Body,
            RequestField::Body => RequestField::Method,
        };
    }

    fn prev_field(&mut self) {
        if self.focus.panel != Panel::Request {
            return;
        }
        self.focus.request_field = match self.focus.request_field {
            RequestField::Method => RequestField::Body,
            RequestField::Url => RequestField::Method,
            RequestField::Headers => RequestField::Url,
            RequestField::Body => RequestField::Headers,
        };
    }
}
