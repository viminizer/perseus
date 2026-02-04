use ratatui::layout::Rect;

pub struct AppLayout {
    pub request_area: Rect,
    pub response_area: Rect,
}

impl AppLayout {
    pub fn new(_area: Rect) -> Self {
        Self {
            request_area: Rect::default(),
            response_area: Rect::default(),
        }
    }
}
