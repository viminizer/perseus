use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
    pub request_area: Rect,
    pub response_area: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

        Self {
            request_area: chunks[0],
            response_area: chunks[1],
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
