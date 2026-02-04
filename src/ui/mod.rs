mod layout;
mod widgets;

use layout::{AppLayout, RequestLayout, ResponseLayout};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, Panel, RequestField, ResponseStatus};

pub fn render(frame: &mut Frame, app: &App) {
    let layout = AppLayout::new(frame.area());
    let request_layout = RequestLayout::new(layout.request_area);

    render_request_panel(frame, app, &request_layout);
    render_response_panel(frame, app, layout.response_area);
}

fn is_field_focused(app: &App, field: RequestField) -> bool {
    app.focus.panel == Panel::Request && app.focus.request_field == field
}

fn field_border_color(app: &App, field: RequestField) -> Color {
    if is_field_focused(app, field) {
        Color::Yellow
    } else {
        Color::White
    }
}

fn render_request_panel(frame: &mut Frame, app: &App, layout: &RequestLayout) {
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border_color(app, RequestField::Method)))
        .title("Method");
    let method_text = Paragraph::new(Line::from(app.request.method.as_str()))
        .style(Style::default().fg(field_border_color(app, RequestField::Method)))
        .block(method_block);
    frame.render_widget(method_text, layout.method_area);

    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border_color(app, RequestField::Url)))
        .title("URL");
    let url_text = Paragraph::new(Line::from(app.request.url.as_str()))
        .style(Style::default().fg(field_border_color(app, RequestField::Url)))
        .block(url_block);
    frame.render_widget(url_text, layout.url_area);

    if is_field_focused(app, RequestField::Url) {
        let cursor_x = layout.url_area.x + 1 + app.request.url_cursor as u16;
        let cursor_y = layout.url_area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    let headers_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border_color(app, RequestField::Headers)))
        .title("Headers");
    let headers_text = Paragraph::new(app.request.headers.as_str())
        .style(Style::default().fg(field_border_color(app, RequestField::Headers)))
        .block(headers_block);
    frame.render_widget(headers_text, layout.headers_area);

    let body_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border_color(app, RequestField::Body)))
        .title("Body");
    let body_text = Paragraph::new(app.request.body.as_str())
        .style(Style::default().fg(field_border_color(app, RequestField::Body)))
        .block(body_block);
    frame.render_widget(body_text, layout.body_area);
}

fn render_response_panel(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus.panel == Panel::Response {
        Color::Yellow
    } else {
        Color::White
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title("Response");

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    match &app.response {
        ResponseStatus::Empty => {
            let hint = Paragraph::new("Press Enter to send request")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(hint, inner_area);
        }
        ResponseStatus::Loading => {
            let loading = Paragraph::new("Loading...")
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(loading, inner_area);
        }
        ResponseStatus::Error(msg) => {
            let error = Paragraph::new(msg.as_str())
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error, inner_area);
        }
        ResponseStatus::Success(data) => {
            let response_layout = ResponseLayout::new(inner_area);
            render_response_content(frame, data, &response_layout);
        }
    }
}

fn render_response_content(
    frame: &mut Frame,
    data: &crate::app::ResponseData,
    layout: &ResponseLayout,
) {
    let status_color = if data.status >= 200 && data.status < 300 {
        Color::Green
    } else if data.status >= 400 {
        Color::Red
    } else {
        Color::Yellow
    };

    let status_line = Line::from(vec![
        Span::styled(
            format!("{} {}", data.status, data.status_text),
            Style::default().fg(status_color),
        ),
        Span::raw(" "),
        Span::styled(
            format!("({}ms)", data.duration_ms),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_block = Block::default().borders(Borders::ALL).title("Status");
    let status_widget = Paragraph::new(status_line).block(status_block);
    frame.render_widget(status_widget, layout.status_area);

    let headers_text: Vec<Line> = data
        .headers
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{}: ", k), Style::default().fg(Color::Cyan)),
                Span::raw(v),
            ])
        })
        .collect();

    let headers_block = Block::default().borders(Borders::ALL).title("Headers");
    let headers_widget = Paragraph::new(headers_text).block(headers_block);
    frame.render_widget(headers_widget, layout.headers_area);

    let body_block = Block::default().borders(Borders::ALL).title("Body");
    let body_widget = Paragraph::new(data.body.as_str()).block(body_block);
    frame.render_widget(body_widget, layout.body_area);
}
