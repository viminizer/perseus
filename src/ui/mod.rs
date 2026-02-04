mod layout;
mod widgets;

use layout::{AppLayout, RequestLayout};
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let layout = AppLayout::new(frame.area());
    let request_layout = RequestLayout::new(layout.request_area);

    render_request_panel(frame, app, &request_layout);
    render_response_panel(frame, layout.response_area);
}

fn render_request_panel(frame: &mut Frame, app: &App, layout: &RequestLayout) {
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("Method");
    let method_text = Paragraph::new(Line::from(app.request.method.as_str()))
        .block(method_block);
    frame.render_widget(method_text, layout.method_area);

    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("URL");
    let url_text = Paragraph::new(Line::from(app.request.url.as_str()))
        .block(url_block);
    frame.render_widget(url_text, layout.url_area);

    let headers_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("Headers");
    let headers_text = Paragraph::new(app.request.headers.as_str())
        .block(headers_block);
    frame.render_widget(headers_text, layout.headers_area);

    let body_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("Body");
    let body_text = Paragraph::new(app.request.body.as_str())
        .block(body_block);
    frame.render_widget(body_text, layout.body_area);
}

fn render_response_panel(frame: &mut Frame, area: ratatui::layout::Rect) {
    let response_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title("Response");

    frame.render_widget(response_block, area);
}
