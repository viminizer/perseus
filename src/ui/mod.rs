mod layout;
mod widgets;

use layout::AppLayout;
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, _app: &App) {
    let layout = AppLayout::new(frame.area());

    let request_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("Request");

    let response_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("Response");

    frame.render_widget(request_block, layout.request_area);
    frame.render_widget(response_block, layout.response_area);
}
