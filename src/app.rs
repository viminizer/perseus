use std::io::stdout;
use std::panic;

use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use tokio::sync::mpsc;
use tui_textarea::{Input, TextArea};

use crate::vim::{Transition, Vim, VimMode};
use crate::{http, ui};

#[derive(Debug, Clone, Default)]
pub enum ResponseStatus {
    #[default]
    Empty,
    Loading,
    Success(ResponseData),
    Error(String),
    Cancelled,
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
pub enum AppMode {
    #[default]
    Navigation,
    Editing,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
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
    Send,
    Headers,
    Body,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FocusState {
    pub panel: Panel,
    pub request_field: RequestField,
}

pub struct RequestState {
    pub method: HttpMethod,
    pub url_editor: TextArea<'static>,
    pub headers_editor: TextArea<'static>,
    pub body_editor: TextArea<'static>,
}

impl RequestState {
    pub fn new() -> Self {
        let mut url_editor = TextArea::default();
        url_editor.set_cursor_line_style(Style::default());
        url_editor.set_placeholder_text("Enter URL...");

        let mut headers_editor = TextArea::default();
        headers_editor.set_cursor_line_style(Style::default());
        headers_editor.set_placeholder_text("Key: Value");

        let mut body_editor = TextArea::default();
        body_editor.set_cursor_line_style(Style::default());
        body_editor.set_placeholder_text("Request body...");

        Self {
            method: HttpMethod::default(),
            url_editor,
            headers_editor,
            body_editor,
        }
    }

    pub fn url_text(&self) -> String {
        self.url_editor.lines().join("")
    }

    pub fn headers_text(&self) -> String {
        self.headers_editor.lines().join("\n")
    }

    pub fn body_text(&self) -> String {
        self.body_editor.lines().join("\n")
    }

    pub fn active_editor(&mut self, field: RequestField) -> Option<&mut TextArea<'static>> {
        match field {
            RequestField::Url => Some(&mut self.url_editor),
            RequestField::Headers => Some(&mut self.headers_editor),
            RequestField::Body => Some(&mut self.body_editor),
            RequestField::Method | RequestField::Send => None,
        }
    }
}

pub struct App {
    running: bool,
    pub request: RequestState,
    pub focus: FocusState,
    pub response: ResponseStatus,
    pub client: Client,
    pub app_mode: AppMode,
    pub vim: Vim,
    pub response_scroll: u16,
    pub loading_tick: u8,
    pub show_help: bool,
    pub show_method_popup: bool,
    pub method_popup_index: usize,
    pub sidebar_visible: bool,
    request_handle: Option<tokio::task::AbortHandle>,
    pub response_editor: TextArea<'static>,
}

impl App {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            running: true,
            request: RequestState::new(),
            focus: FocusState::default(),
            response: ResponseStatus::Empty,
            client,
            app_mode: AppMode::Navigation,
            vim: Vim::new(VimMode::Normal),
            response_scroll: 0,
            loading_tick: 0,
            show_help: false,
            show_method_popup: false,
            method_popup_index: 0,
            sidebar_visible: true,
            request_handle: None,
            response_editor: {
                let mut editor = TextArea::default();
                editor.set_cursor_line_style(Style::default());
                editor
            },
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

    pub fn prepare_editors(&mut self) {
        let is_editing = self.app_mode == AppMode::Editing;
        let focused_field = self.focus.request_field;
        let in_request = self.focus.panel == Panel::Request;

        let url_focused = in_request && focused_field == RequestField::Url;
        let headers_focused = in_request && focused_field == RequestField::Headers;
        let body_focused = in_request && focused_field == RequestField::Body;

        let url_border = if url_focused { Color::Green } else { Color::White };
        let headers_border = if headers_focused { Color::Green } else { Color::White };
        let body_border = if body_focused { Color::Green } else { Color::White };

        self.request.url_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(url_border)),
        );
        self.request.headers_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(headers_border))
                .title("Headers"),
        );
        self.request.body_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(body_border))
                .title("Body"),
        );

        // Set cursor style based on mode
        let cursor_style = if is_editing && url_focused {
            self.vim_cursor_style()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        self.request.url_editor.set_cursor_style(cursor_style);

        let cursor_style = if is_editing && headers_focused {
            self.vim_cursor_style()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        self.request.headers_editor.set_cursor_style(cursor_style);

        let cursor_style = if is_editing && body_focused {
            self.vim_cursor_style()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        self.request.body_editor.set_cursor_style(cursor_style);

        // Response editor block/cursor
        let response_editing = is_editing && self.focus.panel == Panel::Response;
        let response_border = if response_editing {
            Color::Green
        } else {
            Color::White
        };
        self.response_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(response_border))
                .title("Body"),
        );
        let response_cursor = if response_editing {
            self.vim_cursor_style()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        self.response_editor.set_cursor_style(response_cursor);
    }

    fn vim_cursor_style(&self) -> Style {
        match self.vim.mode {
            VimMode::Normal => Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::REVERSED),
            VimMode::Insert => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            VimMode::Visual => Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::REVERSED),
            VimMode::Operator(_) => Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::REVERSED),
        }
    }

    async fn event_loop(&mut self) -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let (tx, mut rx) = mpsc::channel::<Result<ResponseData, String>>(1);

        while self.running {
            self.prepare_editors();
            terminal.draw(|frame| {
                ui::render(frame, self);
            })?;

            if let Ok(result) = rx.try_recv() {
                if matches!(self.response, ResponseStatus::Loading) {
                    self.response = match result {
                        Ok(data) => ResponseStatus::Success(data),
                        Err(e) => ResponseStatus::Error(e),
                    };
                    self.response_scroll = 0;
                    if let ResponseStatus::Success(ref data) = self.response {
                        let mut lines: Vec<String> =
                            data.body.lines().map(String::from).collect();
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        self.response_editor = TextArea::new(lines);
                        self.response_editor.set_cursor_line_style(Style::default());
                    }
                }
                self.request_handle = None;
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
        match self.app_mode {
            AppMode::Navigation => self.handle_navigation_mode(key, tx),
            AppMode::Editing => self.handle_editing_mode(key, tx),
        }
    }

    fn handle_navigation_mode(
        &mut self,
        key: KeyEvent,
        tx: mpsc::Sender<Result<ResponseData, String>>,
    ) {
        // Handle help overlay first
        if self.show_help {
            if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
                self.show_help = false;
            }
            return;
        }

        // Handle method popup navigation when open
        if self.show_method_popup {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.method_popup_index =
                        (self.method_popup_index + 1) % HttpMethod::ALL.len();
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

        let in_request = self.focus.panel == Panel::Request;
        let in_response = self.focus.panel == Panel::Response;

        // Ctrl+E toggles sidebar
        if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.sidebar_visible = !self.sidebar_visible;
            return;
        }

        // Ctrl+R: send request or cancel if loading
        if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(self.response, ResponseStatus::Loading) {
                self.cancel_request();
            } else {
                self.send_request(tx);
            }
            return;
        }

        // Ctrl+h/l: horizontal navigation in input row
        if in_request && key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('h') => {
                    self.prev_horizontal();
                    return;
                }
                KeyCode::Char('l') => {
                    self.next_horizontal();
                    return;
                }
                KeyCode::Char('j') => {
                    self.next_vertical();
                    return;
                }
                KeyCode::Char('k') => {
                    self.prev_vertical();
                    return;
                }
                _ => {}
            }
        }

        // Response panel scrolling with j/k
        if in_response {
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

        // Arrow keys + bare hjkl for navigation in Request panel
        if in_request {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    self.prev_horizontal();
                    return;
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.next_horizontal();
                    return;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.prev_vertical();
                    return;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.next_vertical();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            // Enter: activate focused element
            KeyCode::Enter => {
                if in_request {
                    match self.focus.request_field {
                        RequestField::Method => {
                            self.method_popup_index = self.request.method.index();
                            self.show_method_popup = true;
                        }
                        RequestField::Send => {
                            if matches!(self.response, ResponseStatus::Loading) {
                                self.cancel_request();
                            } else {
                                self.send_request(tx);
                            }
                        }
                        RequestField::Url | RequestField::Headers | RequestField::Body => {
                            self.enter_editing(VimMode::Normal);
                        }
                    }
                } else if in_response
                    && matches!(self.response, ResponseStatus::Success(_))
                {
                    self.enter_editing(VimMode::Normal);
                }
            }
            // i on editable field: enter vim insert mode directly
            KeyCode::Char('i') => {
                if in_request && self.is_editable_field() {
                    self.enter_editing(VimMode::Insert);
                } else if in_response
                    && matches!(self.response, ResponseStatus::Success(_))
                {
                    self.enter_editing(VimMode::Normal);
                }
            }
            KeyCode::Tab => self.cycle_panel(),
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
    }

    fn handle_editing_mode(
        &mut self,
        key: KeyEvent,
        tx: mpsc::Sender<Result<ResponseData, String>>,
    ) {
        // Ctrl+R: send request or cancel if loading, even in editing mode
        if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(self.response, ResponseStatus::Loading) {
                self.cancel_request();
            } else {
                self.send_request(tx);
            }
            return;
        }

        let input: Input = key.into();
        let is_response = self.focus.panel == Panel::Response;

        let transition = if is_response {
            self.vim
                .transition(input, &mut self.response_editor, false)
        } else {
            let field = self.focus.request_field;
            let single_line = field == RequestField::Url;
            if let Some(textarea) = self.request.active_editor(field) {
                self.vim.transition(input, textarea, single_line)
            } else {
                self.exit_editing();
                return;
            }
        };

        match transition {
            Transition::ExitField => {
                self.exit_editing();
            }
            Transition::Mode(new_mode) => {
                let textarea = if is_response {
                    &mut self.response_editor
                } else {
                    self.request
                        .active_editor(self.focus.request_field)
                        .unwrap()
                };
                self.vim = std::mem::replace(&mut self.vim, Vim::new(VimMode::Normal))
                    .apply_transition(Transition::Mode(new_mode), textarea);
                self.update_terminal_cursor();
            }
            Transition::Pending(pending_input) => {
                let textarea = if is_response {
                    &mut self.response_editor
                } else {
                    self.request
                        .active_editor(self.focus.request_field)
                        .unwrap()
                };
                self.vim = std::mem::replace(&mut self.vim, Vim::new(VimMode::Normal))
                    .apply_transition(Transition::Pending(pending_input), textarea);
            }
            Transition::Nop => {}
        }
    }

    fn enter_editing(&mut self, mode: VimMode) {
        self.app_mode = AppMode::Editing;
        self.vim = Vim::new(mode);
        self.update_terminal_cursor();
    }

    fn exit_editing(&mut self) {
        self.app_mode = AppMode::Navigation;
        self.vim = Vim::new(VimMode::Normal);
        let _ = stdout().execute(SetCursorStyle::DefaultUserShape);
    }

    fn update_terminal_cursor(&self) {
        let style = match self.vim.mode {
            VimMode::Normal => SetCursorStyle::SteadyBlock,
            VimMode::Insert => SetCursorStyle::BlinkingUnderScore,
            VimMode::Visual => SetCursorStyle::SteadyBlock,
            VimMode::Operator(_) => SetCursorStyle::SteadyBlock,
        };
        let _ = stdout().execute(style);
    }

    fn send_request(&mut self, tx: mpsc::Sender<Result<ResponseData, String>>) {
        let url = self.request.url_text();
        if url.is_empty() {
            self.response = ResponseStatus::Error("URL is required".to_string());
            return;
        }

        if matches!(self.response, ResponseStatus::Loading) {
            return;
        }

        self.response = ResponseStatus::Loading;

        let client = self.client.clone();
        let method = self.request.method;
        let headers = self.request.headers_text();
        let body = self.request.body_text();

        let handle = tokio::spawn(async move {
            let result = http::send_request(&client, method, &url, &headers, &body).await;
            let _ = tx.send(result).await;
        });
        self.request_handle = Some(handle.abort_handle());
    }

    fn cancel_request(&mut self) {
        if let Some(handle) = self.request_handle.take() {
            handle.abort();
        }
        self.response = ResponseStatus::Cancelled;
    }

    fn is_editable_field(&self) -> bool {
        matches!(
            self.focus.request_field,
            RequestField::Url | RequestField::Headers | RequestField::Body
        )
    }

    fn cycle_panel(&mut self) {
        self.focus.panel = match self.focus.panel {
            Panel::Sidebar => Panel::Request,
            Panel::Request => Panel::Response,
            Panel::Response => Panel::Request,
        };
    }

    fn next_horizontal(&mut self) {
        if self.focus.panel != Panel::Request {
            return;
        }
        self.focus.request_field = match self.focus.request_field {
            RequestField::Method => RequestField::Url,
            RequestField::Url => RequestField::Send,
            RequestField::Send => RequestField::Method,
            // If on Headers/Body, move to input row
            RequestField::Headers | RequestField::Body => RequestField::Url,
        };
    }

    fn prev_horizontal(&mut self) {
        if self.focus.panel != Panel::Request {
            return;
        }
        self.focus.request_field = match self.focus.request_field {
            RequestField::Method => RequestField::Send,
            RequestField::Url => RequestField::Method,
            RequestField::Send => RequestField::Url,
            RequestField::Headers | RequestField::Body => RequestField::Url,
        };
    }

    fn next_vertical(&mut self) {
        if self.focus.panel != Panel::Request {
            return;
        }
        self.focus.request_field = match self.focus.request_field {
            RequestField::Method | RequestField::Url | RequestField::Send => RequestField::Headers,
            RequestField::Headers => RequestField::Body,
            RequestField::Body => RequestField::Url,
        };
    }

    fn prev_vertical(&mut self) {
        if self.focus.panel != Panel::Request {
            return;
        }
        self.focus.request_field = match self.focus.request_field {
            RequestField::Method | RequestField::Url | RequestField::Send => RequestField::Body,
            RequestField::Headers => RequestField::Url,
            RequestField::Body => RequestField::Headers,
        };
    }
}
