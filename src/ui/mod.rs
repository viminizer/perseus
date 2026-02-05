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

use crate::app::{App, InputMode, Panel, RequestField, ResponseStatus};

pub fn render(frame: &mut Frame, app: &App) {
    let layout = AppLayout::new(frame.area());
    let request_layout = RequestLayout::new(layout.request_area);

    render_request_panel(frame, app, &request_layout);
    render_response_panel(frame, app, layout.response_area);
    render_status_bar(frame, app, layout.status_bar);
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
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame_idx = (app.loading_tick as usize / 4) % spinner_frames.len();
            let loading = Paragraph::new(format!("{} Sending request...", spinner_frames[frame_idx]))
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
            render_response_content(frame, data, &response_layout, app.response_scroll);
        }
    }
}

fn render_response_content(
    frame: &mut Frame,
    data: &crate::app::ResponseData,
    layout: &ResponseLayout,
    scroll_offset: u16,
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
    let is_json = is_json_response(&data.headers, &data.body);
    let body_lines = if is_json {
        colorize_json(&data.body)
    } else {
        data.body.lines().map(|l| Line::from(l.to_string())).collect()
    };
    let body_widget = Paragraph::new(body_lines)
        .block(body_block)
        .scroll((scroll_offset, 0));
    frame.render_widget(body_widget, layout.body_area);
}

fn is_json_response(headers: &[(String, String)], body: &str) -> bool {
    let has_json_content_type = headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("content-type") && v.contains("application/json")
    });
    if has_json_content_type {
        return true;
    }
    let trimmed = body.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

fn colorize_json(json: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    let mut chars = json.chars().peekable();
    let mut in_string = false;
    let mut is_key = false;
    let mut current_token = String::new();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_string => {
                in_string = true;
                is_key = current_spans.is_empty()
                    || current_spans
                        .last()
                        .is_some_and(|s| s.content.ends_with(['{', ',', '\n']));
                current_token.push(c);
            }
            '"' if in_string => {
                current_token.push(c);
                let color = if is_key { Color::Cyan } else { Color::Green };
                current_spans.push(Span::styled(
                    std::mem::take(&mut current_token),
                    Style::default().fg(color),
                ));
                in_string = false;
            }
            '\n' => {
                if !current_token.is_empty() {
                    current_spans.push(Span::raw(std::mem::take(&mut current_token)));
                }
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }
            _ if in_string => {
                current_token.push(c);
            }
            ':' | ',' | '{' | '}' | '[' | ']' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
            }
            c if c.is_whitespace() => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
            }
            _ => {
                current_token.push(c);
            }
        }
    }

    if !current_token.is_empty() {
        let span = colorize_token(&current_token);
        current_spans.push(span);
    }
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

fn colorize_token(token: &str) -> Span<'static> {
    let trimmed = token.trim();
    if trimmed == "true" || trimmed == "false" || trimmed == "null" {
        Span::styled(token.to_string(), Style::default().fg(Color::Magenta))
    } else if trimmed.parse::<f64>().is_ok() {
        Span::styled(token.to_string(), Style::default().fg(Color::Yellow))
    } else {
        Span::raw(token.to_string())
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (mode_text, mode_color) = match app.input_mode {
        InputMode::Normal => ("[NORMAL]", Color::Cyan),
        InputMode::Insert => ("[INSERT]", Color::Yellow),
    };

    let panel_info = match app.focus.panel {
        Panel::Request => {
            let field = match app.focus.request_field {
                RequestField::Method => "Method",
                RequestField::Url => "URL",
                RequestField::Headers => "Headers",
                RequestField::Body => "Body",
            };
            format!("Request > {}", field)
        }
        Panel::Response => "Response".to_string(),
    };

    let hints = match app.input_mode {
        InputMode::Normal => "i:insert  j/k:nav  Tab:panel  Enter:send  q:quit",
        InputMode::Insert => "Esc:normal  Enter:newline",
    };

    let status_line = Line::from(vec![
        Span::styled(mode_text, Style::default().fg(mode_color)),
        Span::raw("  "),
        Span::raw(panel_info),
        Span::raw("  │  "),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
    ]);

    let status_bar = Paragraph::new(status_line)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
