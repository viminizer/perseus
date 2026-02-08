use std::collections::HashSet;
use std::io::stdout;
use std::panic;
use std::time::{Duration, Instant};

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
use serde_json::Value;
use tokio::sync::mpsc;
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

use crate::clipboard::ClipboardProvider;
use crate::storage::{
    self, CollectionStore, NodeKind, PostmanHeader, PostmanItem, PostmanRequest, ProjectInfo,
    ProjectTree, TreeNode,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
}

impl ResponseTab {
    pub fn label(self) -> &'static str {
        match self {
            ResponseTab::Body => "Body",
            ResponseTab::Headers => "Headers",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestTab {
    #[default]
    Headers,
    Body,
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
    Sidebar,
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

    pub fn from_str(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            _ => HttpMethod::Get,
        }
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

#[derive(Debug, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(value: String) -> Self {
        Self {
            cursor: value.len(),
            value,
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.value.remove(self.cursor);
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        self.value.remove(self.cursor);
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarPopup {
    Add(TextInput),
    Rename(TextInput),
    Search(TextInput),
    ProjectSwitch { index: usize },
    Move { index: usize, candidates: Vec<Uuid> },
    DeleteConfirm,
}

#[derive(Debug, Clone)]
pub struct SidebarState {
    pub selection_id: Option<Uuid>,
    pub expanded: HashSet<Uuid>,
    pub search_query: String,
    pub popup: Option<SidebarPopup>,
}

#[derive(Debug, Clone)]
pub struct SidebarLine {
    pub id: Uuid,
    pub depth: usize,
    pub prefix: String,
    pub marker: String,
    pub label: String,
    pub kind: NodeKind,
    pub method: Option<HttpMethod>,
}

pub struct RequestState {
    pub method: HttpMethod,
    pub url_editor: TextArea<'static>,
    pub headers_editor: TextArea<'static>,
    pub body_editor: TextArea<'static>,
}

#[derive(Clone, Copy)]
enum YankTarget {
    Request,
    ResponseBody,
    ResponseHeaders,
}

impl RequestState {
    pub fn new() -> Self {
        let mut url_editor = TextArea::default();
        configure_editor(&mut url_editor, "Enter URL...");

        let mut headers_editor = TextArea::default();
        configure_editor(&mut headers_editor, "Key: Value");

        let mut body_editor = TextArea::default();
        configure_editor(&mut body_editor, "Request body...");

        Self {
            method: HttpMethod::default(),
            url_editor,
            headers_editor,
            body_editor,
        }
    }

    pub fn set_contents(&mut self, method: HttpMethod, url: String, headers: String, body: String) {
        self.method = method;
        let url_lines = if url.is_empty() { vec![String::new()] } else { vec![url] };
        let header_lines = if headers.is_empty() {
            vec![String::new()]
        } else {
            headers.lines().map(|l| l.to_string()).collect()
        };
        let body_lines = if body.is_empty() {
            vec![String::new()]
        } else {
            body.lines().map(|l| l.to_string()).collect()
        };

        self.url_editor = TextArea::new(url_lines);
        configure_editor(&mut self.url_editor, "Enter URL...");
        self.headers_editor = TextArea::new(header_lines);
        configure_editor(&mut self.headers_editor, "Key: Value");
        self.body_editor = TextArea::new(body_lines);
        configure_editor(&mut self.body_editor, "Request body...");
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

fn configure_editor(editor: &mut TextArea<'static>, placeholder: &str) {
    editor.set_cursor_line_style(Style::default());
    editor.set_placeholder_text(placeholder);
}

pub struct App {
    running: bool,
    pub request: RequestState,
    pub focus: FocusState,
    pub response: ResponseStatus,
    pub response_tab: ResponseTab,
    pub request_tab: RequestTab,
    pub client: Client,
    pub app_mode: AppMode,
    pub vim: Vim,
    pub response_scroll: u16,
    pub loading_tick: u8,
    pub show_help: bool,
    pub show_method_popup: bool,
    pub method_popup_index: usize,
    pub sidebar_visible: bool,
    pub sidebar_width: u16,
    pub collection: CollectionStore,
    pub project_list: Vec<ProjectInfo>,
    pub sidebar_tree: ProjectTree,
    pub sidebar: SidebarState,
    pub active_project_id: Uuid,
    pub current_request_id: Option<Uuid>,
    pub request_dirty: bool,
    clipboard_toast: Option<(String, Instant)>,
    request_handle: Option<tokio::task::AbortHandle>,
    clipboard: ClipboardProvider,
    last_yank_request: String,
    last_yank_response: String,
    last_yank_response_headers: String,
    pub response_editor: TextArea<'static>,
    pub response_headers_editor: TextArea<'static>,
}

impl App {
    const CLIPBOARD_TOAST_DURATION: Duration = Duration::from_secs(2);

    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let mut collection = CollectionStore::load_or_init().map_err(anyhow::Error::msg)?;
        if collection.collection.item.is_empty() {
            let root_name = collection
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Perseus")
                .to_string();
            let _ = collection
                .add_project(root_name)
                .map_err(anyhow::Error::msg)?;
            collection.save().map_err(anyhow::Error::msg)?;
        }

        let project_list = collection.list_projects();
        if project_list.is_empty() {
            return Err(anyhow::anyhow!("No projects found in collection"));
        }

        let ui_state = storage::load_ui_state()
            .map_err(anyhow::Error::msg)?
            .unwrap_or_else(|| {
            storage::UiState::new(project_list[0].id.to_string(), 32)
        });
        let mut active_project_id =
            Uuid::parse_str(&ui_state.active_project_id).unwrap_or(project_list[0].id);
        if !project_list.iter().any(|p| p.id == active_project_id) {
            active_project_id = project_list[0].id;
        }

        let mut created_request_id: Option<Uuid> = None;
        if !collection_has_requests(&collection.collection.item) {
            let req = PostmanRequest::new("GET".to_string(), String::new(), Vec::new(), None);
            let new_id = collection
                .add_request(active_project_id, "New Request".to_string(), req)
                .map_err(anyhow::Error::msg)?;
            collection.save().map_err(anyhow::Error::msg)?;
            created_request_id = Some(new_id);
        }

        let sidebar_width = clamp_sidebar_width(ui_state.sidebar_width);
        let sidebar_tree = collection
            .build_tree(active_project_id)
            .map_err(anyhow::Error::msg)?;

        let mut expanded = HashSet::new();
        expanded.insert(active_project_id);

        let sidebar = SidebarState {
            selection_id: Some(active_project_id),
            expanded,
            search_query: String::new(),
            popup: None,
        };

        collection
            .write_all_request_files()
            .map_err(anyhow::Error::msg)?;

        let mut app = Self {
            running: true,
            request: RequestState::new(),
            focus: FocusState::default(),
            response: ResponseStatus::Empty,
            response_tab: ResponseTab::default(),
            request_tab: RequestTab::default(),
            client,
            app_mode: AppMode::Navigation,
            vim: Vim::new(VimMode::Normal),
            response_scroll: 0,
            loading_tick: 0,
            show_help: false,
            show_method_popup: false,
            method_popup_index: 0,
            sidebar_visible: true,
            sidebar_width,
            collection,
            project_list,
            sidebar_tree,
            sidebar,
            active_project_id,
            current_request_id: None,
            request_dirty: false,
            clipboard_toast: None,
            request_handle: None,
            clipboard: ClipboardProvider::new(),
            last_yank_request: String::new(),
            last_yank_response: String::new(),
            last_yank_response_headers: String::new(),
            response_editor: {
                let mut editor = TextArea::default();
                editor.set_cursor_line_style(Style::default());
                editor
            },
            response_headers_editor: {
                let mut editor = TextArea::default();
                editor.set_cursor_line_style(Style::default());
                editor
            },
        };

        if let Some(request_id) = created_request_id {
            app.sidebar.selection_id = Some(request_id);
            app.expand_sidebar_ancestors(request_id);
            app.open_request(request_id);
        }

        app.persist_ui_state();
        Ok(app)
    }

    pub async fn run(&mut self) -> Result<()> {
        self.install_panic_hook();
        self.setup_terminal()?;

        let result = self.event_loop().await;

        self.restore_terminal()?;
        result
    }

    pub fn clipboard_toast_message(&self) -> Option<&str> {
        match &self.clipboard_toast {
            Some((msg, at)) if at.elapsed() <= Self::CLIPBOARD_TOAST_DURATION => Some(msg.as_str()),
            _ => None,
        }
    }

    fn set_clipboard_toast(&mut self, msg: impl Into<String>) {
        self.clipboard_toast = Some((msg.into(), Instant::now()));
    }

    fn persist_ui_state(&self) {
        let state = storage::UiState::new(self.active_project_id.to_string(), self.sidebar_width);
        if let Err(err) = storage::save_ui_state(&state) {
            eprintln!("Failed to save UI state: {}", err);
        }
    }

    fn rebuild_sidebar_tree(&mut self) {
        if let Ok(tree) = self.collection.build_tree(self.active_project_id) {
            self.sidebar_tree = tree;
        }
        self.sidebar
            .expanded
            .retain(|id| self.sidebar_tree.nodes.contains_key(id));
        if !self.sidebar.expanded.contains(&self.active_project_id) {
            self.sidebar.expanded.insert(self.active_project_id);
        }
        if let Some(selected) = self.sidebar.selection_id {
            if !self.sidebar_tree.nodes.contains_key(&selected) {
                self.sidebar.selection_id = Some(self.active_project_id);
            }
        } else {
            self.sidebar.selection_id = Some(self.active_project_id);
        }
    }

    fn expand_sidebar_ancestors(&mut self, id: Uuid) {
        let mut current = Some(id);
        while let Some(node_id) = current {
            if let Some(node) = self.sidebar_tree.node(node_id) {
                if matches!(node.kind, NodeKind::Folder | NodeKind::Project) {
                    self.sidebar.expanded.insert(node_id);
                }
                current = node.parent_id;
            } else {
                break;
            }
        }
    }

    fn focus_sidebar(&mut self) {
        if !self.sidebar_visible {
            self.sidebar_visible = true;
        }
        if let Some(request_id) = self.current_request_id {
            if self.sidebar_tree.nodes.contains_key(&request_id) {
                self.sidebar.selection_id = Some(request_id);
                self.expand_sidebar_ancestors(request_id);
            } else {
                self.sidebar.selection_id = Some(self.active_project_id);
            }
        } else {
            self.sidebar.selection_id = Some(self.active_project_id);
        }
        self.focus.panel = Panel::Sidebar;
        self.app_mode = AppMode::Sidebar;
    }

    pub fn sidebar_lines(&self) -> Vec<SidebarLine> {
        if !self.sidebar.search_query.is_empty() {
            return self.sidebar_search_lines();
        }

        let mut lines = Vec::new();
        self.collect_sidebar_lines(self.sidebar_tree.root_id, &[], true, true, &mut lines);
        lines
    }

    fn sidebar_search_lines(&self) -> Vec<SidebarLine> {
        let mut lines = Vec::new();
        let query = self.sidebar.search_query.to_lowercase();
        for (id, node) in &self.sidebar_tree.nodes {
            if node.kind == NodeKind::Project {
                continue;
            }
            if node.name.to_lowercase().contains(&query) {
                let path = self.sidebar_tree.path_for(*id).join("/");
                let method = if node.kind == NodeKind::Request {
                    self.collection
                        .get_item(*id)
                        .and_then(|item| item.request.as_ref())
                        .map(|request| HttpMethod::from_str(&request.method))
                } else {
                    None
                };
                lines.push(SidebarLine {
                    id: *id,
                    depth: 0,
                    prefix: String::new(),
                    marker: String::new(),
                    label: path,
                    kind: node.kind,
                    method,
                });
            }
        }
        lines.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        lines
    }

    fn collect_sidebar_lines(
        &self,
        id: Uuid,
        ancestors_last: &[bool],
        is_last: bool,
        is_root: bool,
        out: &mut Vec<SidebarLine>,
    ) {
        if let Some(node) = self.sidebar_tree.node(id) {
            let is_expanded = self.sidebar.expanded.contains(&id);
            let marker = match node.kind {
                NodeKind::Project | NodeKind::Folder => {
                    if is_expanded { "▾" } else { "▸" }
                }
                NodeKind::Request => "•",
            };
            let method = if node.kind == NodeKind::Request {
                self.collection
                    .get_item(node.id)
                    .and_then(|item| item.request.as_ref())
                    .map(|request| HttpMethod::from_str(&request.method))
            } else {
                None
            };
            let prefix = if is_root {
                String::new()
            } else {
                sidebar_tree_prefix(ancestors_last, is_last)
            };
            out.push(SidebarLine {
                id,
                depth: if is_root { 0 } else { ancestors_last.len() + 1 },
                prefix,
                marker: marker.to_string(),
                label: node.name.clone(),
                kind: node.kind,
                method,
            });
            if matches!(node.kind, NodeKind::Project | NodeKind::Folder) && is_expanded {
                let mut next_ancestors = ancestors_last.to_vec();
                if !is_root {
                    next_ancestors.push(is_last);
                }
                for (index, child) in node.children.iter().enumerate() {
                    let child_is_last = index + 1 == node.children.len();
                    self.collect_sidebar_lines(
                        *child,
                        &next_ancestors,
                        child_is_last,
                        false,
                        out,
                    );
                }
            }
        }
    }

    pub fn sidebar_selected_index(&self, lines: &[SidebarLine]) -> usize {
        let Some(selected) = self.sidebar.selection_id else {
            return 0;
        };
        lines
            .iter()
            .position(|line| line.id == selected)
            .unwrap_or(0)
    }

    fn sidebar_move_selection(&mut self, delta: i32) {
        let lines = self.sidebar_lines();
        if lines.is_empty() {
            return;
        }
        let mut index = self.sidebar_selected_index(&lines) as i32;
        index = (index + delta).clamp(0, (lines.len() - 1) as i32);
        self.sidebar.selection_id = Some(lines[index as usize].id);
    }

    fn sidebar_selected_node(&self) -> Option<&TreeNode> {
        self.sidebar
            .selection_id
            .and_then(|id| self.sidebar_tree.node(id))
    }

    fn sidebar_selected_id(&self) -> Option<Uuid> {
        self.sidebar.selection_id
    }

    fn save_current_request_if_dirty(&mut self) {
        if !self.request_dirty {
            return;
        }
        let Some(request_id) = self.current_request_id else {
            return;
        };
        if let Err(err) = self.save_request_by_id(request_id) {
            self.response = ResponseStatus::Error(err);
        } else {
            self.request_dirty = false;
        }
    }

    fn save_request_by_id(&mut self, request_id: Uuid) -> Result<(), String> {
        let request = self.build_postman_request();
        self.collection.update_request(request_id, request)?;
        self.collection.save()?;
        if let Some(parent_id) = self
            .sidebar_tree
            .node(request_id)
            .and_then(|node| node.parent_id)
        {
            self.collection
                .save_request_file(request_id, parent_id, self.active_project_id)?;
        }
        Ok(())
    }

    fn build_postman_request(&self) -> PostmanRequest {
        let method = self.request.method.as_str().to_string();
        let url = self.request.url_text();
        let headers = storage::parse_headers(&self.request.headers_text());
        let body_raw = self.request.body_text();
        let body = if body_raw.trim().is_empty() {
            None
        } else {
            Some(body_raw)
        };
        PostmanRequest::new(method, url, headers, body)
    }

    fn open_request(&mut self, request_id: Uuid) {
        self.save_current_request_if_dirty();
        if let Some(item) = self.collection.get_item(request_id) {
            if let Some(request) = &item.request {
                let method = HttpMethod::from_str(&request.method);
                let url = extract_url(&request.url);
                let headers = headers_to_text(&request.header);
                let body = request
                    .body
                    .as_ref()
                    .and_then(|b| b.raw.clone())
                    .unwrap_or_default();
                self.request.set_contents(method, url, headers, body);
                self.current_request_id = Some(request_id);
                self.request_dirty = false;
                self.focus.panel = Panel::Request;
                self.focus.request_field = RequestField::Url;
            }
        }
    }

    fn open_project_switcher(&mut self) {
        let index = self
            .project_list
            .iter()
            .position(|p| p.id == self.active_project_id)
            .unwrap_or(0);
        self.sidebar.popup = Some(SidebarPopup::ProjectSwitch { index });
        self.focus.panel = Panel::Sidebar;
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.sidebar_move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.sidebar_move_selection(-1),
            KeyCode::Char('h') => self.sidebar_collapse_or_parent(),
            KeyCode::Char('l') | KeyCode::Enter => self.sidebar_expand_or_open(),
            KeyCode::Char('a') => self.sidebar.popup = Some(SidebarPopup::Add(TextInput::new(String::new()))),
            KeyCode::Char('r') => self.open_rename_popup(),
            KeyCode::Char('d') => self.sidebar.popup = Some(SidebarPopup::DeleteConfirm),
            KeyCode::Char('D') => {
                if let Err(err) = self.duplicate_selected() {
                    self.response = ResponseStatus::Error(err);
                }
            }
            KeyCode::Char('m') => self.open_move_popup(),
            KeyCode::Char('c') => self.copy_selected_path(),
            KeyCode::Char('/') => {
                let input = TextInput::new(self.sidebar.search_query.clone());
                self.sidebar.popup = Some(SidebarPopup::Search(input));
            }
            KeyCode::Char('[') => self.outdent_selected(),
            KeyCode::Char(']') => self.indent_selected(),
            KeyCode::Char('H') => self.collapse_all(),
            KeyCode::Char('L') => self.expand_all(),
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('q') => {
                self.save_current_request_if_dirty();
                self.running = false;
            }
            KeyCode::Esc => {
                if !self.sidebar.search_query.is_empty() {
                    self.sidebar.search_query.clear();
                }
            }
            _ => {}
        }
    }

    fn handle_sidebar_popup(&mut self, key: KeyEvent) {
        let mut popup = match self.sidebar.popup.take() {
            Some(popup) => popup,
            None => return,
        };
        let mut close = false;

        match &mut popup {
            SidebarPopup::Add(input) => {
                if key.code == KeyCode::Enter {
                    if let Err(err) = self.handle_add_input(&input.value) {
                        self.response = ResponseStatus::Error(err);
                    }
                    close = true;
                } else if key.code == KeyCode::Esc {
                    close = true;
                } else {
                    handle_text_input(input, key);
                }
            }
            SidebarPopup::Rename(input) => {
                if key.code == KeyCode::Enter {
                    if let Err(err) = self.rename_selected(input.value.clone()) {
                        self.response = ResponseStatus::Error(err);
                    }
                    close = true;
                } else if key.code == KeyCode::Esc {
                    close = true;
                } else {
                    handle_text_input(input, key);
                }
            }
            SidebarPopup::Search(input) => {
                if key.code == KeyCode::Enter {
                    self.sidebar.search_query = input.value.clone();
                    close = true;
                } else if key.code == KeyCode::Esc {
                    self.sidebar.search_query.clear();
                    close = true;
                } else {
                    handle_text_input(input, key);
                    self.sidebar.search_query = input.value.clone();
                }
            }
            SidebarPopup::ProjectSwitch { index } => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.project_list.is_empty() {
                        *index = (*index + 1) % self.project_list.len();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if !self.project_list.is_empty() {
                        if *index == 0 {
                            *index = self.project_list.len() - 1;
                        } else {
                            *index -= 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(project) = self.project_list.get(*index) {
                        self.set_active_project(project.id);
                    }
                    close = true;
                }
                KeyCode::Esc => close = true,
                _ => {}
            },
            SidebarPopup::Move { index, candidates } => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !candidates.is_empty() {
                        *index = (*index + 1) % candidates.len();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if !candidates.is_empty() {
                        if *index == 0 {
                            *index = candidates.len() - 1;
                        } else {
                            *index -= 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(dest_id) = candidates.get(*index).copied() {
                        if let Err(err) = self.move_selected(dest_id) {
                            self.response = ResponseStatus::Error(err);
                        }
                    }
                    close = true;
                }
                KeyCode::Esc => close = true,
                _ => {}
            },
            SidebarPopup::DeleteConfirm => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    if let Err(err) = self.delete_selected() {
                        self.response = ResponseStatus::Error(err);
                    }
                    close = true;
                }
                KeyCode::Char('n') | KeyCode::Esc => close = true,
                _ => {}
            },
        }

        if close {
            self.sidebar.popup = None;
        } else {
            self.sidebar.popup = Some(popup);
        }
    }

    fn open_rename_popup(&mut self) {
        if let Some(node) = self.sidebar_selected_node() {
            let input = TextInput::new(node.name.clone());
            self.sidebar.popup = Some(SidebarPopup::Rename(input));
        }
    }

    fn handle_add_input(&mut self, input: &str) -> Result<(), String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let (folders, request) = parse_add_path(trimmed);
        let mut parent_id = self.add_parent_id();

        for folder in folders {
            if let Some(existing) = self.find_child_folder(parent_id, &folder) {
                parent_id = existing;
            } else {
                parent_id = self.collection.add_folder(parent_id, folder)?;
            }
        }

        if let Some(request_name) = request {
            let req = PostmanRequest::new("GET".to_string(), String::new(), Vec::new(), None);
            let new_id = self
                .collection
                .add_request(parent_id, request_name, req)?;
            self.collection.save()?;
            self.collection
                .save_request_file(new_id, parent_id, self.active_project_id)?;
            self.refresh_after_collection_change();
            self.sidebar.selection_id = Some(new_id);
            self.open_request(new_id);
        } else {
            self.collection.save()?;
            self.refresh_after_collection_change();
            self.sidebar.selection_id = Some(parent_id);
        }
        Ok(())
    }

    fn rename_selected(&mut self, name: String) -> Result<(), String> {
        let Some(id) = self.sidebar_selected_id() else {
            return Ok(());
        };
        self.collection.rename_item(id, name)?;
        self.collection.save()?;
        self.refresh_after_collection_change();
        Ok(())
    }

    fn delete_selected(&mut self) -> Result<(), String> {
        let Some(id) = self.sidebar_selected_id() else {
            return Ok(());
        };
        let kind = self
            .sidebar_tree
            .node(id)
            .map(|n| n.kind)
            .unwrap_or(NodeKind::Folder);
        let was_active_project = id == self.active_project_id;
        self.collection.delete_item(id)?;
        self.collection.save()?;
        self.project_list = self.collection.list_projects();
        if kind == NodeKind::Project && self.project_list.is_empty() {
            let root_name = self
                .collection
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Perseus")
                .to_string();
            let new_id = self.collection.add_project(root_name)?;
            self.collection.save()?;
            self.project_list = self.collection.list_projects();
            self.active_project_id = new_id;
        } else if was_active_project {
            if let Some(first) = self.project_list.first() {
                self.active_project_id = first.id;
            }
        }
        self.rebuild_sidebar_tree();
        self.persist_ui_state();

        if let Some(current) = self.current_request_id {
            if current == id {
                self.request = RequestState::new();
                self.current_request_id = None;
                self.request_dirty = false;
            }
        }

        Ok(())
    }

    fn duplicate_selected(&mut self) -> Result<(), String> {
        let Some(id) = self.sidebar_selected_id() else {
            return Ok(());
        };
        let new_id = self.collection.duplicate_item(id)?;
        self.collection.save()?;
        self.refresh_after_collection_change();
        self.sidebar.selection_id = Some(new_id);
        Ok(())
    }

    fn move_selected(&mut self, dest_id: Uuid) -> Result<(), String> {
        let Some(id) = self.sidebar_selected_id() else {
            return Ok(());
        };
        if self.sidebar_tree.is_descendant(id, dest_id) {
            return Err("Cannot move into a descendant".to_string());
        }
        let Some(node) = self.sidebar_tree.node(id) else {
            return Ok(());
        };
        if node.kind == NodeKind::Project {
            return Err("Projects cannot be moved".to_string());
        }
        self.collection.move_item(id, dest_id)?;
        self.collection.save()?;
        self.refresh_after_collection_change();
        self.sidebar.selection_id = Some(id);
        Ok(())
    }

    fn open_move_popup(&mut self) {
        let Some(selected) = self.sidebar_selected_id() else {
            return;
        };
        if let Some(node) = self.sidebar_tree.node(selected) {
            if node.kind == NodeKind::Project {
                return;
            }
        }
        let mut candidates = Vec::new();
        for (id, node) in &self.sidebar_tree.nodes {
            if *id == selected {
                continue;
            }
            if node.kind == NodeKind::Request {
                continue;
            }
            if self.sidebar_tree.is_descendant(selected, *id) {
                continue;
            }
            candidates.push(*id);
        }
        candidates.sort_by(|a, b| {
            let ap = self.sidebar_tree.path_for(*a).join("/");
            let bp = self.sidebar_tree.path_for(*b).join("/");
            ap.to_lowercase().cmp(&bp.to_lowercase())
        });
        if candidates.is_empty() {
            return;
        }
        self.sidebar.popup = Some(SidebarPopup::Move { index: 0, candidates });
    }

    fn copy_selected_path(&mut self) {
        let Some(id) = self.sidebar_selected_id() else {
            return;
        };
        let path = self.sidebar_tree.path_for(id).join("/");
        if let Err(_) = self.clipboard.set_text(path) {
            self.set_clipboard_toast("Clipboard write failed");
        } else {
            self.set_clipboard_toast("Copied path");
        }
    }

    fn sidebar_expand_or_open(&mut self) {
        let Some(node) = self.sidebar_selected_node() else {
            return;
        };
        match node.kind {
            NodeKind::Request => {
                self.open_request(node.id);
                self.app_mode = AppMode::Navigation;
            }
            NodeKind::Folder | NodeKind::Project => {
                if !self.sidebar.expanded.contains(&node.id) {
                    self.sidebar.expanded.insert(node.id);
                }
            }
        }
    }

    fn sidebar_collapse_or_parent(&mut self) {
        let Some(node) = self.sidebar_selected_node() else {
            return;
        };
        let node_id = node.id;
        let node_kind = node.kind;
        let node_parent = node.parent_id;
        let is_expanded = self.sidebar.expanded.contains(&node_id);
        match node_kind {
            NodeKind::Request => {
                if let Some(parent) = node_parent {
                    self.sidebar.selection_id = Some(parent);
                }
            }
            NodeKind::Folder | NodeKind::Project => {
                if is_expanded {
                    self.sidebar.expanded.remove(&node_id);
                } else if let Some(parent) = node_parent {
                    self.sidebar.selection_id = Some(parent);
                }
            }
        }
    }

    fn collapse_all(&mut self) {
        self.sidebar.expanded.clear();
    }

    fn expand_all(&mut self) {
        self.sidebar.expanded = self
            .sidebar_tree
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                if node.kind == NodeKind::Request {
                    None
                } else {
                    Some(*id)
                }
            })
            .collect();
    }

    fn indent_selected(&mut self) {
        let Some(selected) = self.sidebar_selected_id() else {
            return;
        };
        let Some(node) = self.sidebar_tree.node(selected) else {
            return;
        };
        let Some(parent_id) = node.parent_id else {
            return;
        };
        let Some(parent) = self.sidebar_tree.node(parent_id) else {
            return;
        };
        let siblings = &parent.children;
        let index = siblings.iter().position(|id| *id == selected).unwrap_or(0);
        if index == 0 {
            return;
        }
        let candidate_id = siblings[index - 1];
        if let Some(candidate) = self.sidebar_tree.node(candidate_id) {
            if candidate.kind == NodeKind::Folder || candidate.kind == NodeKind::Project {
                if let Err(err) = self.move_selected(candidate_id) {
                    self.response = ResponseStatus::Error(err);
                }
            }
        }
    }

    fn outdent_selected(&mut self) {
        let Some(selected) = self.sidebar_selected_id() else {
            return;
        };
        let Some(node) = self.sidebar_tree.node(selected) else {
            return;
        };
        let Some(parent_id) = node.parent_id else {
            return;
        };
        let Some(parent) = self.sidebar_tree.node(parent_id) else {
            return;
        };
        let Some(grand_parent_id) = parent.parent_id else {
            return;
        };
        if let Err(err) = self.move_selected(grand_parent_id) {
            self.response = ResponseStatus::Error(err);
        }
    }

    fn set_active_project(&mut self, project_id: Uuid) {
        if self.active_project_id == project_id {
            return;
        }
        self.save_current_request_if_dirty();
        self.active_project_id = project_id;
        self.rebuild_sidebar_tree();
        self.sidebar.selection_id = Some(project_id);
        self.sidebar.search_query.clear();
        self.persist_ui_state();
    }

    fn refresh_after_collection_change(&mut self) {
        self.project_list = self.collection.list_projects();
        self.rebuild_sidebar_tree();
        self.persist_ui_state();
        if let Err(err) = self.collection.write_all_request_files() {
            self.response = ResponseStatus::Error(err);
        }
    }

    fn add_parent_id(&self) -> Uuid {
        if let Some(selected) = self.sidebar_selected_node() {
            match selected.kind {
                NodeKind::Request => selected.parent_id.unwrap_or(self.active_project_id),
                NodeKind::Folder | NodeKind::Project => selected.id,
            }
        } else {
            self.active_project_id
        }
    }

    fn find_child_folder(&self, parent_id: Uuid, name: &str) -> Option<Uuid> {
        let parent = self.sidebar_tree.node(parent_id)?;
        for child in &parent.children {
            if let Some(node) = self.sidebar_tree.node(*child) {
                if (node.kind == NodeKind::Folder || node.kind == NodeKind::Project)
                    && node.name == name
                {
                    return Some(*child);
                }
            }
        }
        None
    }

    fn active_yank_target(&self) -> Option<YankTarget> {
        match self.focus.panel {
            Panel::Response => match self.response_tab {
                ResponseTab::Body => Some(YankTarget::ResponseBody),
                ResponseTab::Headers => Some(YankTarget::ResponseHeaders),
            },
            Panel::Request => match self.focus.request_field {
                RequestField::Url | RequestField::Headers | RequestField::Body => {
                    Some(YankTarget::Request)
                }
                RequestField::Method | RequestField::Send => None,
            },
            Panel::Sidebar => None,
        }
    }

    fn update_last_yank(&mut self, target: YankTarget, text: String) {
        match target {
            YankTarget::Request => self.last_yank_request = text,
            YankTarget::ResponseBody => self.last_yank_response = text,
            YankTarget::ResponseHeaders => self.last_yank_response_headers = text,
        }
    }

    fn sync_clipboard_from_active_yank(&mut self) {
        let mut new_yank: Option<String> = None;
        match self.focus.panel {
            Panel::Response => match self.response_tab {
                ResponseTab::Body => {
                    let yank = self.response_editor.yank_text();
                    if self.last_yank_response != yank {
                        self.last_yank_response = yank.clone();
                        new_yank = Some(yank);
                    }
                }
                ResponseTab::Headers => {
                    let yank = self.response_headers_editor.yank_text();
                    if self.last_yank_response_headers != yank {
                        self.last_yank_response_headers = yank.clone();
                        new_yank = Some(yank);
                    }
                }
            },
            Panel::Request => {
                if let Some(textarea) = self.request.active_editor(self.focus.request_field) {
                    let yank = textarea.yank_text();
                    if self.last_yank_request != yank {
                        self.last_yank_request = yank.clone();
                        new_yank = Some(yank);
                    }
                }
            }
            Panel::Sidebar => {}
        }

        if let Some(yank) = new_yank {
            if let Err(_) = self.clipboard.set_text(yank) {
                self.set_clipboard_toast("Clipboard write failed");
            }
        }
    }

    fn handle_clipboard_paste_shortcut(&mut self) {
        let target = match self.active_yank_target() {
            Some(target) => target,
            None => return,
        };

        let clipboard_text = match self.clipboard.get_text() {
            Ok(text) => Some(text),
            Err(_) => {
                self.set_clipboard_toast("Clipboard read failed; using internal yank");
                None
            }
        };

        let mut last_yank_update: Option<(YankTarget, String)> = None;
        let mut exit_to_normal = false;

        match target {
            YankTarget::Request => {
                if let Some(textarea) = self.request.active_editor(self.focus.request_field) {
                    if let Some(text) = clipboard_text.as_ref() {
                        textarea.set_yank_text(text.clone());
                        if self.vim.mode == VimMode::Insert {
                            textarea.insert_str(text.as_str());
                        } else {
                            textarea.paste();
                            if matches!(self.vim.mode, VimMode::Visual | VimMode::Operator(_)) {
                                exit_to_normal = true;
                            }
                        }
                        last_yank_update = Some((target, text.clone()));
                    } else if self.vim.mode == VimMode::Insert {
                        let fallback = textarea.yank_text();
                        if !fallback.is_empty() {
                            textarea.insert_str(fallback);
                        }
                    } else {
                        textarea.paste();
                        if matches!(self.vim.mode, VimMode::Visual | VimMode::Operator(_)) {
                            exit_to_normal = true;
                        }
                    }
                }
            }
            YankTarget::ResponseBody => {
                let textarea = &mut self.response_editor;
                if let Some(text) = clipboard_text.as_ref() {
                    textarea.set_yank_text(text.clone());
                    if self.vim.mode == VimMode::Insert {
                        textarea.insert_str(text.as_str());
                    } else {
                        textarea.paste();
                        if matches!(self.vim.mode, VimMode::Visual | VimMode::Operator(_)) {
                            exit_to_normal = true;
                        }
                    }
                    last_yank_update = Some((target, text.clone()));
                } else if self.vim.mode == VimMode::Insert {
                    let fallback = textarea.yank_text();
                    if !fallback.is_empty() {
                        textarea.insert_str(fallback);
                    }
                } else {
                    textarea.paste();
                    if matches!(self.vim.mode, VimMode::Visual | VimMode::Operator(_)) {
                        exit_to_normal = true;
                    }
                }
            }
            YankTarget::ResponseHeaders => {
                let textarea = &mut self.response_headers_editor;
                if let Some(text) = clipboard_text.as_ref() {
                    textarea.set_yank_text(text.clone());
                    if self.vim.mode == VimMode::Insert {
                        textarea.insert_str(text.as_str());
                    } else {
                        textarea.paste();
                        if matches!(self.vim.mode, VimMode::Visual | VimMode::Operator(_)) {
                            exit_to_normal = true;
                        }
                    }
                    last_yank_update = Some((target, text.clone()));
                } else if self.vim.mode == VimMode::Insert {
                    let fallback = textarea.yank_text();
                    if !fallback.is_empty() {
                        textarea.insert_str(fallback);
                    }
                } else {
                    textarea.paste();
                    if matches!(self.vim.mode, VimMode::Visual | VimMode::Operator(_)) {
                        exit_to_normal = true;
                    }
                }
            }
        }

        if let Some((target, text)) = last_yank_update {
            self.update_last_yank(target, text);
        }

        if exit_to_normal {
            self.vim = Vim::new(VimMode::Normal);
            self.update_terminal_cursor();
        }
    }

    fn handle_clipboard_copy_shortcut(&mut self) {
        let target = match self.active_yank_target() {
            Some(target) => target,
            None => return,
        };

        let mut yank: Option<String> = None;
        let mut exit_visual = false;

        match target {
            YankTarget::Request => {
                if let Some(textarea) = self.request.active_editor(self.focus.request_field) {
                    if textarea.is_selecting() {
                        textarea.copy();
                        yank = Some(textarea.yank_text());
                        if self.vim.mode == VimMode::Visual {
                            exit_visual = true;
                        }
                    }
                }
            }
            YankTarget::ResponseBody => {
                let textarea = &mut self.response_editor;
                if textarea.is_selecting() {
                    textarea.copy();
                    yank = Some(textarea.yank_text());
                    if self.vim.mode == VimMode::Visual {
                        exit_visual = true;
                    }
                }
            }
            YankTarget::ResponseHeaders => {
                let textarea = &mut self.response_headers_editor;
                if textarea.is_selecting() {
                    textarea.copy();
                    yank = Some(textarea.yank_text());
                    if self.vim.mode == VimMode::Visual {
                        exit_visual = true;
                    }
                }
            }
        }

        if let Some(text) = yank {
            self.update_last_yank(target, text.clone());
            if let Err(_) = self.clipboard.set_text(text) {
                self.set_clipboard_toast("Clipboard write failed");
            }
        }

        if exit_visual {
            self.vim = Vim::new(VimMode::Normal);
            self.update_terminal_cursor();
        }
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

        self.request.url_editor.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(url_border)),
        );
        self.request
            .headers_editor
            .set_block(Block::default().borders(Borders::NONE));
        self.request
            .body_editor
            .set_block(Block::default().borders(Borders::NONE));

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
        self.response_editor.set_block(Block::default().borders(Borders::NONE));
        self.response_headers_editor
            .set_block(Block::default().borders(Borders::NONE));
        let response_cursor = if response_editing {
            self.vim_cursor_style()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        self.response_editor.set_cursor_style(response_cursor);
        self.response_headers_editor
            .set_cursor_style(response_cursor);
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
                    self.response_tab = ResponseTab::Body;
                    if let ResponseStatus::Success(ref data) = self.response {
                        let mut lines: Vec<String> =
                            data.body.lines().map(String::from).collect();
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        self.response_editor = TextArea::new(lines);
                        self.response_editor.set_cursor_line_style(Style::default());
                        let mut header_lines: Vec<String> = data
                            .headers
                            .iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect();
                        if header_lines.is_empty() {
                            header_lines.push(String::new());
                        }
                        self.response_headers_editor = TextArea::new(header_lines);
                        self.response_headers_editor
                            .set_cursor_line_style(Style::default());
                        self.last_yank_response = self.response_editor.yank_text();
                        self.last_yank_response_headers = self.response_headers_editor.yank_text();
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
            AppMode::Sidebar => self.handle_sidebar_mode(key),
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

        if self.sidebar.popup.is_some() {
            self.handle_sidebar_popup(key);
            return;
        }

        let in_request = self.focus.panel == Panel::Request;
        let in_response = self.focus.panel == Panel::Response;
        let in_sidebar = self.focus.panel == Panel::Sidebar;

        // Ctrl+E toggles sidebar
        if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.sidebar_visible = !self.sidebar_visible;
            if self.sidebar_visible {
                self.focus_sidebar();
            } else {
                if self.focus.panel == Panel::Sidebar {
                    self.focus.panel = Panel::Request;
                    self.focus.request_field = RequestField::Url;
                }
                if matches!(self.app_mode, AppMode::Sidebar) {
                    self.app_mode = AppMode::Navigation;
                }
            }
            return;
        }

        if key.code == KeyCode::Char('e') && key.modifiers.is_empty() {
            self.focus_sidebar();
            return;
        }

        // Ctrl+P: project switcher
        if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.sidebar_visible {
                self.open_project_switcher();
            }
            return;
        }

        // Ctrl+[ / Ctrl+]: resize sidebar
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('[') => {
                    self.sidebar_width = clamp_sidebar_width(self.sidebar_width.saturating_sub(2));
                    self.persist_ui_state();
                    return;
                }
                KeyCode::Char(']') => {
                    self.sidebar_width = clamp_sidebar_width(self.sidebar_width.saturating_add(2));
                    self.persist_ui_state();
                    return;
                }
                _ => {}
            }
        }

        // Ctrl+S: save current request
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(request_id) = self.current_request_id {
                if let Err(err) = self.save_request_by_id(request_id) {
                    self.response = ResponseStatus::Error(err);
                } else {
                    self.request_dirty = false;
                }
            }
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

        // Arrow keys + bare hjkl for navigation
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

        match key.code {
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            // Enter: activate focused element
            KeyCode::Enter => {
                if in_sidebar {
                    self.app_mode = AppMode::Sidebar;
                } else if in_request {
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
                if in_sidebar {
                    self.app_mode = AppMode::Sidebar;
                } else if in_request && self.is_editable_field() {
                    self.enter_editing(VimMode::Insert);
                } else if in_response
                    && matches!(self.response, ResponseStatus::Success(_))
                {
                    self.enter_editing(VimMode::Normal);
                }
            }
            KeyCode::Char('q') => {
                self.save_current_request_if_dirty();
                self.running = false;
            }
            _ => {}
        }
    }

    fn handle_sidebar_mode(&mut self, key: KeyEvent) {
        if self.show_help {
            if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
                self.show_help = false;
            }
            return;
        }

        if key.code == KeyCode::Esc {
            if self.sidebar.popup.is_some() {
                self.sidebar.popup = None;
            }
            if !self.sidebar.search_query.is_empty() {
                self.sidebar.search_query.clear();
            }
            self.app_mode = AppMode::Navigation;
            return;
        }

        if self.sidebar.popup.is_some() {
            self.handle_sidebar_popup(key);
            return;
        }

        self.handle_sidebar_key(key);
    }

    fn handle_editing_mode(
        &mut self,
        key: KeyEvent,
        tx: mpsc::Sender<Result<ResponseData, String>>,
    ) {
        // Ctrl+S: save current request
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(request_id) = self.current_request_id {
                if let Err(err) = self.save_request_by_id(request_id) {
                    self.response = ResponseStatus::Error(err);
                } else {
                    self.request_dirty = false;
                }
            }
            return;
        }

        // Ctrl+R: send request or cancel if loading, even in editing mode
        if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if matches!(self.response, ResponseStatus::Loading) {
                self.cancel_request();
            } else {
                self.send_request(tx);
            }
            return;
        }

        // Enter in URL insert mode: send request (or cancel if loading), then exit editing
        if self.focus.panel == Panel::Request
            && self.focus.request_field == RequestField::Url
            && self.vim.mode == VimMode::Insert
            && key.code == KeyCode::Enter
        {
            if matches!(self.response, ResponseStatus::Loading) {
                self.cancel_request();
            } else {
                self.send_request(tx);
            }
            self.exit_editing();
            return;
        }

        let is_request = self.focus.panel == Panel::Request;
        let is_response = self.focus.panel == Panel::Response;
        let is_request_vim_switch = is_request
            && matches!(self.vim.mode, VimMode::Normal | VimMode::Insert);
        let is_response_vim_switch = is_response
            && matches!(
                self.vim.mode,
                VimMode::Normal | VimMode::Visual | VimMode::Operator(_)
            );

        if is_request_vim_switch {
            match key.code {
                KeyCode::Char('H') => {
                    self.prev_request_tab();
                    return;
                }
                KeyCode::Char('L') => {
                    self.next_request_tab();
                    return;
                }
                _ => {}
            }
        }

        if is_response_vim_switch {
            match key.code {
                KeyCode::Char('H') => {
                    self.prev_response_tab();
                    return;
                }
                KeyCode::Char('L') => {
                    self.next_response_tab();
                    return;
                }
                _ => {}
            }
        }

        let is_clipboard_modifier = key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::SUPER);

        if is_request {
            if key.code != KeyCode::Esc {
                self.request_dirty = true;
            }
        }

        if is_clipboard_modifier && matches!(key.code, KeyCode::Char('v') | KeyCode::Char('V')) {
            self.handle_clipboard_paste_shortcut();
            return;
        }

        if is_clipboard_modifier && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')) {
            self.handle_clipboard_copy_shortcut();
            return;
        }

        if matches!(self.vim.mode, VimMode::Normal | VimMode::Visual)
            && key.modifiers.is_empty()
            && key.code == KeyCode::Char('p')
        {
            if let Some(target) = self.active_yank_target() {
                match self.clipboard.get_text() {
                    Ok(text) => {
                        match target {
                            YankTarget::Request => {
                                if let Some(textarea) =
                                    self.request.active_editor(self.focus.request_field)
                                {
                                    textarea.set_yank_text(text.clone());
                                }
                            }
                            YankTarget::ResponseBody => {
                                self.response_editor.set_yank_text(text.clone());
                            }
                            YankTarget::ResponseHeaders => {
                                self.response_headers_editor.set_yank_text(text.clone());
                            }
                        }
                        self.update_last_yank(target, text);
                    }
                    Err(_) => {
                        self.set_clipboard_toast("Clipboard read failed; using internal yank");
                    }
                }
            }
        }

        let input: Input = key.into();

        let transition = if is_response {
            let response_tab = self.response_tab;
            let vim = &self.vim;
            match response_tab {
                ResponseTab::Body => vim.transition(input, &mut self.response_editor, false),
                ResponseTab::Headers => {
                    vim.transition(input, &mut self.response_headers_editor, false)
                }
            }
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
                if is_response {
                    let response_tab = self.response_tab;
                    let vim = std::mem::replace(&mut self.vim, Vim::new(VimMode::Normal));
                    let new_vim = match response_tab {
                        ResponseTab::Body => vim.apply_transition(
                            Transition::Mode(new_mode),
                            &mut self.response_editor,
                        ),
                        ResponseTab::Headers => vim.apply_transition(
                            Transition::Mode(new_mode),
                            &mut self.response_headers_editor,
                        ),
                    };
                    self.vim = new_vim;
                } else {
                    let textarea = self
                        .request
                        .active_editor(self.focus.request_field)
                        .unwrap();
                    self.vim = std::mem::replace(&mut self.vim, Vim::new(VimMode::Normal))
                        .apply_transition(Transition::Mode(new_mode), textarea);
                }
                self.update_terminal_cursor();
                self.sync_clipboard_from_active_yank();
            }
            Transition::Pending(pending_input) => {
                if is_response {
                    let response_tab = self.response_tab;
                    let vim = std::mem::replace(&mut self.vim, Vim::new(VimMode::Normal));
                    let new_vim = match response_tab {
                        ResponseTab::Body => vim.apply_transition(
                            Transition::Pending(pending_input),
                            &mut self.response_editor,
                        ),
                        ResponseTab::Headers => vim.apply_transition(
                            Transition::Pending(pending_input),
                            &mut self.response_headers_editor,
                        ),
                    };
                    self.vim = new_vim;
                } else {
                    let textarea = self
                        .request
                        .active_editor(self.focus.request_field)
                        .unwrap();
                    self.vim = std::mem::replace(&mut self.vim, Vim::new(VimMode::Normal))
                        .apply_transition(Transition::Pending(pending_input), textarea);
                }
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

    fn next_horizontal(&mut self) {
        match self.focus.panel {
            Panel::Sidebar => {
                self.focus.panel = Panel::Request;
                self.focus.request_field = RequestField::Method;
            }
            Panel::Request => {
                self.focus.request_field = match self.focus.request_field {
                    RequestField::Method => RequestField::Url,
                    RequestField::Url => RequestField::Send,
                    RequestField::Send => RequestField::Method,
                    RequestField::Headers | RequestField::Body => RequestField::Url,
                };
            }
            Panel::Response => {}
        }
    }

    fn prev_horizontal(&mut self) {
        match self.focus.panel {
            Panel::Request => {
                if self.sidebar_visible {
                    self.focus.panel = Panel::Sidebar;
                } else {
                    self.focus.request_field = match self.focus.request_field {
                        RequestField::Method => RequestField::Send,
                        RequestField::Url => RequestField::Method,
                        RequestField::Send => RequestField::Url,
                        RequestField::Headers | RequestField::Body => RequestField::Url,
                    };
                }
            }
            Panel::Sidebar => {}
            Panel::Response => {
                if self.sidebar_visible {
                    self.focus.panel = Panel::Sidebar;
                }
            }
        }
    }

    fn next_vertical(&mut self) {
        match self.focus.panel {
            Panel::Request => {
                self.focus.request_field = match self.focus.request_field {
                    RequestField::Method | RequestField::Url | RequestField::Send => {
                        match self.request_tab {
                            RequestTab::Headers => RequestField::Headers,
                            RequestTab::Body => RequestField::Body,
                        }
                    }
                    RequestField::Headers | RequestField::Body => {
                        self.focus.panel = Panel::Response;
                        return;
                    }
                };
            }
            Panel::Response | Panel::Sidebar => {}
        }
    }

    fn prev_vertical(&mut self) {
        match self.focus.panel {
            Panel::Response => {
                self.focus.panel = Panel::Request;
                self.focus.request_field = match self.request_tab {
                    RequestTab::Headers => RequestField::Headers,
                    RequestTab::Body => RequestField::Body,
                };
            }
            Panel::Request => {
                self.focus.request_field = match self.focus.request_field {
                    RequestField::Method | RequestField::Url | RequestField::Send => {
                        match self.request_tab {
                            RequestTab::Headers => RequestField::Headers,
                            RequestTab::Body => RequestField::Body,
                        }
                    }
                    RequestField::Headers | RequestField::Body => RequestField::Url,
                };
            }
            Panel::Sidebar => {}
        }
    }

    fn next_request_tab(&mut self) {
        self.request_tab = match self.request_tab {
            RequestTab::Headers => RequestTab::Body,
            RequestTab::Body => RequestTab::Headers,
        };

        if self.focus.panel == Panel::Request {
            self.focus.request_field = match self.focus.request_field {
                RequestField::Headers | RequestField::Body => match self.request_tab {
                    RequestTab::Headers => RequestField::Headers,
                    RequestTab::Body => RequestField::Body,
                },
                other => other,
            };
        }
    }

    fn prev_request_tab(&mut self) {
        self.next_request_tab();
    }

    fn next_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Headers,
            ResponseTab::Headers => ResponseTab::Body,
        };
    }

    fn prev_response_tab(&mut self) {
        self.next_response_tab();
    }
}

fn sidebar_tree_prefix(ancestors_last: &[bool], is_last: bool) -> String {
    let mut prefix = String::new();
    for ancestor_last in ancestors_last {
        if *ancestor_last {
            prefix.push_str("  ");
        } else {
            prefix.push_str("│ ");
        }
    }
    if is_last {
        prefix.push_str("└─ ");
    } else {
        prefix.push_str("├─ ");
    }
    prefix
}

fn collection_has_requests(items: &[PostmanItem]) -> bool {
    for item in items {
        if item.request.is_some() {
            return true;
        }
        if !item.item.is_empty() && collection_has_requests(&item.item) {
            return true;
        }
    }
    false
}

fn clamp_sidebar_width(value: u16) -> u16 {
    value.max(28).min(42)
}

fn extract_url(value: &Value) -> String {
    match value {
        Value::String(raw) => raw.clone(),
        Value::Object(map) => map
            .get("raw")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn headers_to_text(headers: &[PostmanHeader]) -> String {
    let mut lines = Vec::new();
    for header in headers {
        if header.key.trim().is_empty() {
            continue;
        }
        lines.push(format!("{}: {}", header.key, header.value));
    }
    lines.join("\n")
}

fn handle_text_input(input: &mut TextInput, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::ALT)
        || key.modifiers.contains(KeyModifiers::SUPER)
    {
        return;
    }
    match key.code {
        KeyCode::Char(ch) => input.insert_char(ch),
        KeyCode::Backspace => input.backspace(),
        KeyCode::Delete => input.delete(),
        KeyCode::Left => input.move_left(),
        KeyCode::Right => input.move_right(),
        KeyCode::Home => input.cursor = 0,
        KeyCode::End => input.cursor = input.value.len(),
        _ => {}
    }
}

fn parse_add_path(raw: &str) -> (Vec<String>, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (Vec::new(), None);
    }
    let trailing = trimmed.ends_with('/');
    let parts: Vec<String> = trimmed
        .split('/')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();
    if parts.is_empty() {
        return (Vec::new(), None);
    }
    if trailing {
        (parts, None)
    } else if parts.len() == 1 {
        (Vec::new(), Some(parts[0].clone()))
    } else {
        let mut folders = parts.clone();
        let request = folders.pop();
        (folders, request)
    }
}
