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
