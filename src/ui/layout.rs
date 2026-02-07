use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
    pub sidebar_area: Rect,
    pub request_area: Rect,
    pub response_area: Rect,
    pub status_bar: Rect,
}

impl AppLayout {
    pub fn new(area: Rect, sidebar_visible: bool) -> Self {
        let vertical = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

        let main_area = vertical[0];
        let status_bar = vertical[1];

        let (sidebar_area, content_area) = if sidebar_visible {
            // Split: sidebar (20 chars or 15%) | main content
            let sidebar_width = std::cmp::min(20, main_area.width * 15 / 100);
            let with_sidebar = Layout::horizontal([
                Constraint::Length(sidebar_width),
                Constraint::Min(1),
            ])
            .split(main_area);
            (with_sidebar[0], with_sidebar[1])
        } else {
            // No sidebar - full width for content
            (Rect::default(), main_area)
        };

        // Main content is vertical: request area (50%) | response area (50%)
        let content_vertical = Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(content_area);

        Self {
            sidebar_area,
            request_area: content_vertical[0],
            response_area: content_vertical[1],
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
    pub tab_area: Rect,
    pub spacer_area: Rect,
    pub content_area: Rect,
}

impl RequestLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::vertical([
            Constraint::Length(1),   // Tabs
            Constraint::Length(1),   // Spacer
            Constraint::Min(3),      // Content (takes remaining space)
        ])
        .split(area);

        Self {
            tab_area: chunks[0],
            spacer_area: chunks[1],
            content_area: chunks[2],
        }
    }
}

pub struct ResponseLayout {
    pub tab_area: Rect,
    pub spacer_area: Rect,
    pub content_area: Rect,
}

impl ResponseLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
        ])
        .split(area);

        Self {
            tab_area: chunks[0],
            spacer_area: chunks[1],
            content_area: chunks[2],
        }
    }
}
