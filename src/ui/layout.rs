use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
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

        let horizontal = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_area);

        Self {
            request_area: horizontal[0],
            response_area: horizontal[1],
            status_bar,
        }
    }
}

pub struct RequestLayout {
    pub method_area: Rect,
    pub url_area: Rect,
    pub headers_area: Rect,
    pub body_area: Rect,
}

impl RequestLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Percentage(30),
            Constraint::Min(3),
        ])
        .split(area);

        Self {
            method_area: chunks[0],
            url_area: chunks[1],
            headers_area: chunks[2],
            body_area: chunks[3],
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
