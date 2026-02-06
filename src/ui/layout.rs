use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
    pub sidebar_area: Rect,
    pub request_area: Rect,
    pub response_area: Rect,
    pub status_bar: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let vertical = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

        let main_area = vertical[0];
        let status_bar = vertical[1];

        // Split: sidebar (20 chars or 15%) | main content
        let sidebar_width = std::cmp::min(20, main_area.width * 15 / 100);
        let with_sidebar = Layout::horizontal([
            Constraint::Length(sidebar_width),
            Constraint::Min(1),
        ])
        .split(main_area);

        let sidebar_area = with_sidebar[0];
        let content_area = with_sidebar[1];

        // Main content splits into request | response
        let horizontal = Layout::horizontal([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(content_area);

        Self {
            sidebar_area,
            request_area: horizontal[0],
            response_area: horizontal[1],
            status_bar,
        }
    }
}

/// Layout for the horizontal request input row: [Method] [URL] [Send]
pub struct RequestInputLayout {
    pub method_area: Rect,
    pub url_area: Rect,
    pub send_area: Rect,
}

impl RequestInputLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::horizontal([
            Constraint::Length(10),  // Method: fits "DELETE" + padding
            Constraint::Min(1),      // URL: fill remaining space
            Constraint::Length(10),  // Send button: fits "[ Send ]"
        ])
        .split(area);

        Self {
            method_area: chunks[0],
            url_area: chunks[1],
            send_area: chunks[2],
        }
    }
}

pub struct RequestLayout {
    pub input_row: RequestInputLayout,
    pub headers_area: Rect,
    pub body_area: Rect,
}

impl RequestLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::vertical([
            Constraint::Length(3),   // Input row (method + url + send)
            Constraint::Length(5),   // Headers
            Constraint::Min(5),      // Body
        ])
        .split(area);

        Self {
            input_row: RequestInputLayout::new(chunks[0]),
            headers_area: chunks[1],
            body_area: chunks[2],
        }
    }
}

pub struct ResponseLayout {
    pub status_area: Rect,
    pub headers_area: Rect,
    pub body_area: Rect,
}

impl ResponseLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Percentage(30),
            Constraint::Min(3),
        ])
        .split(area);

        Self {
            status_area: chunks[0],
            headers_area: chunks[1],
            body_area: chunks[2],
        }
    }
}
