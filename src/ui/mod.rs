mod layout;
mod widgets;

use layout::{AppLayout, RequestLayout, ResponseLayout};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, HttpMethod, Panel, RequestField, ResponseStatus};
use crate::vim::VimMode;

pub fn render(frame: &mut Frame, app: &App) {
    let layout = AppLayout::new(frame.area(), app.sidebar_visible);
    let request_layout = RequestLayout::new(layout.request_area);

    if app.sidebar_visible {
        render_sidebar(frame, layout.sidebar_area);
    }
    render_request_panel(frame, app, &request_layout);
    render_response_panel(frame, app, layout.response_area);
    render_status_bar(frame, app, layout.status_bar);

    if app.show_method_popup {
        render_method_popup(frame, app, request_layout.input_row.method_area);
    }

    if app.show_help {
        render_help_overlay(frame);
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("Collections");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let placeholder = Paragraph::new("No collections yet")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(placeholder, inner);
}

fn render_method_popup(frame: &mut Frame, app: &App, method_area: Rect) {
    let width: u16 = 15;
    let height: u16 = HttpMethod::ALL.len() as u16 + 2;
    let x = method_area.x;
    let y = method_area.y + method_area.height;
    let popup_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Method ");

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let lines: Vec<Line> = HttpMethod::ALL
        .iter()
        .enumerate()
        .map(|(i, method)| {
            let color = method_color(*method);
            let is_selected = i == app.method_popup_index;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(color)
            } else {
                Style::default().fg(color)
            };
            Line::from(Span::styled(format!(" {} ", method.as_str()), style))
        })
        .collect();

    let list = Paragraph::new(lines);
    frame.render_widget(list, inner);
}

fn is_field_focused(app: &App, field: RequestField) -> bool {
    app.focus.panel == Panel::Request && app.focus.request_field == field
}

fn method_color(method: HttpMethod) -> Color {
    match method {
        HttpMethod::Get => Color::Green,
        HttpMethod::Post => Color::Blue,
        HttpMethod::Put => Color::Yellow,
        HttpMethod::Patch => Color::Magenta,
        HttpMethod::Delete => Color::Red,
    }
}

fn render_request_panel(frame: &mut Frame, app: &App, layout: &RequestLayout) {
    // Render Method box with method-specific color
    let method_focused = is_field_focused(app, RequestField::Method);
    let method_col = method_color(app.request.method);
    let method_border = if method_focused { Color::Green } else { Color::DarkGray };
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(method_border));
    let method_text = Paragraph::new(Line::from(app.request.method.as_str()))
        .style(Style::default().fg(method_col))
        .alignment(Alignment::Center)
        .block(method_block);
    frame.render_widget(method_text, layout.input_row.method_area);

    // Render URL editor (TextArea handles its own cursor)
    frame.render_widget(&app.request.url_editor, layout.input_row.url_area);

    // Render Send/Cancel button with focus highlight
    let send_focused = is_field_focused(app, RequestField::Send);
    let is_loading = matches!(app.response, ResponseStatus::Loading);
    let (btn_label, btn_color) = if is_loading {
        ("[ Cancel ]", Color::Red)
    } else {
        ("[ Send ]", Color::Green)
    };
    let send_border_color = if send_focused { Color::Green } else { Color::DarkGray };
    let send_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(send_border_color));
    let send_text = Paragraph::new(Line::from(btn_label))
        .style(Style::default().fg(btn_color))
        .block(send_block);
    frame.render_widget(send_text, layout.input_row.send_area);

    // Render Headers editor (TextArea)
    frame.render_widget(&app.request.headers_editor, layout.headers_area);

    // Render Body editor (TextArea)
    frame.render_widget(&app.request.body_editor, layout.body_area);
}

fn render_response_panel(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus.panel == Panel::Response {
        Color::Green
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
            let hint = Paragraph::new("Press Ctrl+R to send request")
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
            let error_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title("Error");
            let error_inner = error_block.inner(inner_area);
            frame.render_widget(error_block, inner_area);

            let error_lines = vec![Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red)),
                Span::raw(msg.as_str()),
            ])];
            let error_text = Paragraph::new(error_lines)
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            frame.render_widget(error_text, error_inner);
        }
        ResponseStatus::Cancelled => {
            let hint = Paragraph::new("⊘ Request cancelled")
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(hint, inner_area);
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
    let (mode_text, mode_style) = match app.app_mode {
        AppMode::Navigation => (
            " NAVIGATION ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        AppMode::Editing => match app.vim.mode {
            VimMode::Normal => (
                " VIM ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            VimMode::Insert => (
                " INSERT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            VimMode::Visual => (
                " VISUAL ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Magenta)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            VimMode::Operator(_) => (
                " PENDING ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        },
    };

    let panel_info = match app.focus.panel {
        Panel::Sidebar => "Sidebar".to_string(),
        Panel::Request => {
            let field = match app.focus.request_field {
                RequestField::Method => "Method",
                RequestField::Url => "URL",
                RequestField::Send => "Send",
                RequestField::Headers => "Headers",
                RequestField::Body => "Body",
            };
            format!("Request > {}", field)
        }
        Panel::Response => "Response".to_string(),
    };

    let hints = match app.app_mode {
        AppMode::Navigation => {
            "Ctrl+hjkl:nav  Tab:panel  Enter:edit  i:insert  Ctrl+r:send  ?:help  q:quit"
        }
        AppMode::Editing => match app.vim.mode {
            VimMode::Normal => "hjkl:move  w/b/e:word  i/a:insert  v:visual  d/c/y:op  Esc:exit",
            VimMode::Insert => "type text  Esc:normal",
            VimMode::Visual => "motion:select  d:delete  y:yank  c:change  Esc:cancel",
            VimMode::Operator(_) => "motion:complete  Esc:cancel",
        },
    };

    let status_line = Line::from(vec![
        Span::styled(mode_text, mode_style),
        Span::raw("  "),
        Span::raw(panel_info),
        Span::raw("  │  "),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
    ]);

    let status_bar = Paragraph::new(status_line)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}

fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    let width = (area.width as f32 * 0.6) as u16;
    let height = (area.height as f32 * 0.7) as u16;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let help_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, help_area);

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help (press ? to close) ");

    let help_inner = help_block.inner(help_area);
    frame.render_widget(help_block, help_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Navigation Mode",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Ctrl+h/l    Move between fields horizontally"),
        Line::from("  Ctrl+j/k    Move between field rows"),
        Line::from("  Arrow keys  Same as Ctrl+hjkl"),
        Line::from("  Tab         Switch panel (Request/Response)"),
        Line::from("  Enter       Activate field (vim normal mode)"),
        Line::from("  i           Enter field (vim insert mode)"),
        Line::from("  Ctrl+r      Send request"),
        Line::from("  Ctrl+e      Toggle sidebar"),
        Line::from("  j/k         Scroll response (in Response panel)"),
        Line::from("  q / Esc     Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Vim Editing Mode",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  h/j/k/l     Cursor movement"),
        Line::from("  w/b/e       Word forward/back/end"),
        Line::from("  0/^/$       Line start/end"),
        Line::from("  gg/G        Top/bottom"),
        Line::from("  i/a/I/A     Enter insert mode"),
        Line::from("  o/O         New line below/above (multiline)"),
        Line::from("  v/V         Visual / visual line"),
        Line::from("  d/c/y       Delete/change/yank (+ motion)"),
        Line::from("  dd/cc/yy    Operate on line"),
        Line::from("  x/X         Delete char forward/backward"),
        Line::from("  D/C         Delete/change to end of line"),
        Line::from("  p           Paste"),
        Line::from("  u / Ctrl+r  Undo / redo"),
        Line::from("  Esc         Exit to navigation mode"),
    ];

    let help_paragraph = Paragraph::new(help_text);
    frame.render_widget(help_paragraph, help_inner);
}
