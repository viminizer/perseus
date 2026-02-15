mod layout;
mod widgets;

use layout::{AppLayout, RequestInputLayout, RequestLayout, ResponseLayout};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthChar;

use crate::app::{
    App, AppMode, AuthField, AuthType, HttpMethod, Method, Panel, RequestField, RequestTab,
    ResponseBodyRenderCache, ResponseHeadersRenderCache, ResponseStatus, ResponseTab,
    SidebarPopup, WrapCache,
};
use crate::perf;
use crate::storage::NodeKind;
use crate::vim::VimMode;

pub fn render(frame: &mut Frame, app: &mut App) {
    let layout = AppLayout::new(frame.area(), app.sidebar_visible, app.sidebar_width);
    let request_split = Layout::vertical([Constraint::Length(3), Constraint::Min(3)])
        .split(layout.request_area);
    let input_layout = RequestInputLayout::new(request_split[0]);

    if app.sidebar_visible {
        render_sidebar(frame, app, layout.sidebar_area);
    }
    render_request_input_row(frame, app, &input_layout);
    render_request_panel(frame, app, request_split[1]);
    render_response_panel(frame, app, layout.response_area);
    render_status_bar(frame, app, layout.status_bar);

    if app.show_method_popup {
        render_method_popup(frame, app, input_layout.method_area);
    }

    if app.show_auth_type_popup {
        render_auth_type_popup(frame, app, request_split[1]);
    }

    if app.show_help {
        render_help_overlay(frame);
    }
}

fn render_sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = if app.focus.panel == Panel::Sidebar {
        Color::Green
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title("Explorer");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let project_name = app
        .project_list
        .iter()
        .find(|p| p.id == app.active_project_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Project".to_string());

    let search_query = app.sidebar.search_query.clone();
    let selected_id = app.sidebar.selection_id;

    let mut lines: Vec<Line> = Vec::new();
    let header = Line::from(vec![
        Span::styled(
            format!("Project: {}", project_name),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Ctrl+P", Style::default().fg(Color::DarkGray)),
    ]);
    lines.push(header);
    lines.push(Line::from(""));

    if !search_query.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Search: {}", search_query),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
    }

    let width = inner.width as usize;
    {
        let items = app.sidebar_lines();
        if items.is_empty() {
            lines.push(Line::from(Span::styled(
                "No items",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for item in items.iter() {
                let is_selected = Some(item.id) == selected_id;
                let base_style = if is_selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                let mut spans: Vec<Span> = Vec::new();
                let mut text_len: usize = 0;

                let push_span =
                    |content: String, style: Style, spans: &mut Vec<Span>, len: &mut usize| {
                        *len = len.saturating_add(content.chars().count());
                        spans.push(Span::styled(content, style));
                    };

                if !item.prefix.is_empty() {
                    push_span(item.prefix.clone(), base_style, &mut spans, &mut text_len);
                }

                match item.kind {
                    NodeKind::Request => {
                        if let Some(ref method) = item.method {
                            let method_style = base_style.fg(method_color(method));
                            push_span(
                                method.as_str().to_string(),
                                method_style,
                                &mut spans,
                                &mut text_len,
                            );
                            push_span(" ".to_string(), base_style, &mut spans, &mut text_len);
                        }
                        push_span(item.label.clone(), base_style, &mut spans, &mut text_len);
                    }
                    NodeKind::Folder | NodeKind::Project => {
                        let label = if item.marker.is_empty() {
                            item.label.clone()
                        } else {
                            format!("{} {}", item.marker, item.label)
                        };
                        push_span(label, base_style, &mut spans, &mut text_len);
                    }
                }

                let max_width = width.saturating_sub(1);
                if max_width > text_len {
                    let padding = " ".repeat(max_width - text_len);
                    push_span(padding, base_style, &mut spans, &mut text_len);
                }

                lines.push(Line::from(spans));
            }
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);

    if let Some(popup) = &app.sidebar.popup {
        render_sidebar_popup(frame, app, popup, area);
    }
}

fn render_sidebar_popup(frame: &mut Frame, app: &App, popup: &SidebarPopup, area: Rect) {
    let (title, body_lines) = match popup {
        SidebarPopup::Add(input) => (
            "Add",
            vec![
                Line::from("Name or path (folder/req or folder/)"),
                Line::from(""),
                Line::from(render_input_line(input)),
                Line::from(""),
                Line::from("Enter: create  Esc: cancel"),
            ],
        ),
        SidebarPopup::Rename(input) => (
            "Rename",
            vec![
                Line::from("New name"),
                Line::from(""),
                Line::from(render_input_line(input)),
                Line::from(""),
                Line::from("Enter: rename  Esc: cancel"),
            ],
        ),
        SidebarPopup::Search(input) => (
            "Search",
            vec![
                Line::from("Filter items"),
                Line::from(""),
                Line::from(render_input_line(input)),
                Line::from(""),
                Line::from("Enter: apply  Esc: clear"),
            ],
        ),
        SidebarPopup::ProjectSwitch { index } => {
            let mut lines = vec![Line::from("Select project"), Line::from("")];
            for (i, project) in app.project_list.iter().enumerate() {
                let style = if i == *index {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(Span::styled(project.name.clone(), style)));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Enter: switch  Esc: cancel"));
            ("Projects", lines)
        }
        SidebarPopup::Move { index, candidates } => {
            let mut lines = vec![Line::from("Move to"), Line::from("")];
            for (i, id) in candidates.iter().enumerate() {
                let path = app.sidebar_tree.path_for(*id).join("/");
                let style = if i == *index {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(Span::styled(path, style)));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Enter: move  Esc: cancel"));
            ("Move", lines)
        }
        SidebarPopup::DeleteConfirm => (
            "Delete",
            vec![
                Line::from("Delete selected item?"),
                Line::from(""),
                Line::from("y / Enter: confirm"),
                Line::from("n / Esc: cancel"),
            ],
        ),
    };

    let width = std::cmp::min(60, area.width.saturating_sub(4));
    let height = std::cmp::min(10, area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", title));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    let paragraph = Paragraph::new(body_lines);
    frame.render_widget(paragraph, inner);
}

fn render_input_line(input: &crate::app::TextInput) -> Line<'static> {
    let mut text = input.value.clone();
    if input.cursor <= text.len() {
        text.insert(input.cursor, '|');
    } else {
        text.push('|');
    }
    Line::from(Span::styled(
        text,
        Style::default().fg(Color::White).bg(Color::Black),
    ))
}

fn render_method_popup(frame: &mut Frame, app: &App, method_area: Rect) {
    let popup_item_count = HttpMethod::ALL.len() + 1; // 7 standard + "Custom..."
    let width: u16 = 15;
    let height: u16 = popup_item_count as u16 + 2;
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

    let mut lines: Vec<Line> = HttpMethod::ALL
        .iter()
        .enumerate()
        .map(|(i, method)| {
            let m = Method::Standard(*method);
            let color = method_color(&m);
            let is_selected = i == app.method_popup_index;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(color)
            } else {
                Style::default().fg(color)
            };
            Line::from(Span::styled(format!(" {} ", method.as_str()), style))
        })
        .collect();

    // "Custom..." entry or inline text input
    let custom_index = HttpMethod::ALL.len();
    let is_custom_selected = app.method_popup_index == custom_index;

    if app.method_popup_custom_mode {
        let input_text = format!(" {}_ ", app.method_custom_input);
        let style = Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);
        lines.push(Line::from(Span::styled(input_text, style)));
    } else {
        let style = if is_custom_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC)
        };
        lines.push(Line::from(Span::styled(" Custom... ", style)));
    }

    let list = Paragraph::new(lines);
    frame.render_widget(list, inner);
}

fn render_auth_type_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width: u16 = 20;
    let height: u16 = AuthType::ALL.len() as u16 + 2;
    let x = area.x + 2;
    let y = area.y + 2;
    let popup_area = Rect::new(
        x.min(area.right().saturating_sub(width)),
        y.min(area.bottom().saturating_sub(height)),
        width.min(area.width),
        height.min(area.height),
    );

    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Auth Type ");

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let lines: Vec<Line> = AuthType::ALL
        .iter()
        .enumerate()
        .map(|(i, auth_type)| {
            let is_selected = i == app.auth_type_popup_index;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(format!(" {} ", auth_type.as_str()), style))
        })
        .collect();

    let list = Paragraph::new(lines);
    frame.render_widget(list, inner);
}

fn is_field_focused(app: &App, field: RequestField) -> bool {
    app.focus.panel == Panel::Request && app.focus.request_field == field
}

fn method_color(method: &Method) -> Color {
    match method {
        Method::Standard(m) => match m {
            HttpMethod::Get => Color::Green,
            HttpMethod::Post => Color::Blue,
            HttpMethod::Put => Color::Yellow,
            HttpMethod::Patch => Color::Magenta,
            HttpMethod::Delete => Color::Red,
            HttpMethod::Head => Color::Cyan,
            HttpMethod::Options => Color::White,
        },
        Method::Custom(_) => Color::DarkGray,
    }
}

fn render_request_input_row(frame: &mut Frame, app: &App, layout: &RequestInputLayout) {
    // Render Method box with method-specific color
    let method_focused = is_field_focused(app, RequestField::Method);
    let method_col = method_color(&app.request.method);
    let method_border = if method_focused { Color::Green } else { Color::DarkGray };
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(method_border));
    // Truncate method display to fit area (inner width minus padding)
    let display_str = app.request.method.as_str();
    let max_width = layout.method_area.width.saturating_sub(2) as usize; // account for border
    let display = if display_str.len() > max_width {
        format!("{}\u{2026}", &display_str[..max_width.saturating_sub(1)])
    } else {
        display_str.to_string()
    };
    let method_text = Paragraph::new(Line::from(display))
        .style(Style::default().fg(method_col))
        .alignment(Alignment::Center)
        .block(method_block);
    frame.render_widget(method_text, layout.method_area);

    // Render URL editor (TextArea handles its own cursor)
    frame.render_widget(&app.request.url_editor, layout.url_area);

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
    frame.render_widget(send_text, layout.send_area);
}

fn render_request_panel(frame: &mut Frame, app: &App, area: Rect) {
    let request_panel_focused = app.focus.panel == Panel::Request
        && matches!(
            app.focus.request_field,
            RequestField::Headers | RequestField::Auth | RequestField::Body
        );
    let border_color = if request_panel_focused {
        Color::Green
    } else {
        Color::White
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title("Request");

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let layout = RequestLayout::new(inner_area);

    // Render Request tabs
    render_request_tab_bar(frame, app, layout.tab_area);
    frame.render_widget(Paragraph::new(""), layout.spacer_area);

    // Render active Request editor (TextArea)
    match app.request_tab {
        RequestTab::Headers => {
            frame.render_widget(&app.request.headers_editor, layout.content_area);
        }
        RequestTab::Auth => {
            render_auth_panel(frame, app, layout.content_area);
        }
        RequestTab::Body => {
            frame.render_widget(&app.request.body_editor, layout.content_area);
        }
    }
}

fn render_request_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let request_panel_focused = app.focus.panel == Panel::Request
        && matches!(
            app.focus.request_field,
            RequestField::Headers | RequestField::Auth | RequestField::Body
        );
    let active_color = if request_panel_focused {
        Color::Green
    } else {
        Color::White
    };
    let active_style = Style::default()
        .fg(active_color)
        .add_modifier(Modifier::UNDERLINED);
    let inactive_style = Style::default().fg(Color::DarkGray);

    let auth_label = match app.request.auth_type {
        AuthType::NoAuth => "Auth".to_string(),
        AuthType::Bearer => "Auth (Bearer)".to_string(),
        AuthType::Basic => "Auth (Basic)".to_string(),
        AuthType::ApiKey => "Auth (API Key)".to_string(),
    };

    let tabs_line = Line::from(vec![
        Span::styled(
            "Headers",
            if app.request_tab == RequestTab::Headers {
                active_style
            } else {
                inactive_style
            },
        ),
        Span::styled(" | ", inactive_style),
        Span::styled(
            auth_label,
            if app.request_tab == RequestTab::Auth {
                active_style
            } else {
                inactive_style
            },
        ),
        Span::styled(" | ", inactive_style),
        Span::styled(
            "Body",
            if app.request_tab == RequestTab::Body {
                active_style
            } else {
                inactive_style
            },
        ),
    ]);

    let tabs_widget = Paragraph::new(tabs_line);
    frame.render_widget(tabs_widget, area);
}

fn render_auth_panel(frame: &mut Frame, app: &App, area: Rect) {
    use crate::app::ApiKeyLocation;

    let auth_focused = app.focus.panel == Panel::Request
        && app.focus.request_field == RequestField::Auth;

    // Layout: type selector row (1) + separator (1) + content (fill)
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);

    // Row 1: Auth type selector
    let type_label = format!("Type: [{}]", app.request.auth_type.as_str());
    let type_focused = auth_focused && app.focus.auth_field == AuthField::AuthType;
    let type_style = if type_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::White)
    };
    frame.render_widget(Paragraph::new(type_label).style(type_style), chunks[0]);

    // Separator
    let sep_style = Style::default().fg(Color::DarkGray);
    let sep_line = "─".repeat(area.width as usize);
    frame.render_widget(Paragraph::new(sep_line).style(sep_style), chunks[1]);

    // Content area — per auth type
    let content_area = chunks[2];
    match app.request.auth_type {
        AuthType::NoAuth => {
            let msg = Paragraph::new("No authentication configured")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(msg, content_area);
        }
        AuthType::Bearer => {
            let field_chunks = Layout::vertical([
                Constraint::Length(1), // label
                Constraint::Min(0),   // textarea
            ])
            .split(content_area);

            let label_focused =
                auth_focused && app.focus.auth_field == AuthField::Token;
            let label_style = if label_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Cyan)
            };
            frame.render_widget(
                Paragraph::new("Token:").style(label_style),
                field_chunks[0],
            );
            frame.render_widget(&app.request.auth_token_editor, field_chunks[1]);
        }
        AuthType::Basic => {
            let field_chunks = Layout::vertical([
                Constraint::Length(1), // username label
                Constraint::Length(3), // username textarea
                Constraint::Length(1), // password label
                Constraint::Min(0),   // password textarea
            ])
            .split(content_area);

            let username_focused =
                auth_focused && app.focus.auth_field == AuthField::Username;
            let password_focused =
                auth_focused && app.focus.auth_field == AuthField::Password;

            let u_style = if username_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Cyan)
            };
            frame.render_widget(
                Paragraph::new("Username:").style(u_style),
                field_chunks[0],
            );
            frame.render_widget(&app.request.auth_username_editor, field_chunks[1]);

            let p_style = if password_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Cyan)
            };
            frame.render_widget(
                Paragraph::new("Password:").style(p_style),
                field_chunks[2],
            );
            frame.render_widget(&app.request.auth_password_editor, field_chunks[3]);
        }
        AuthType::ApiKey => {
            let field_chunks = Layout::vertical([
                Constraint::Length(1), // key name label
                Constraint::Length(2), // key name textarea
                Constraint::Length(1), // key value label
                Constraint::Length(2), // key value textarea
                Constraint::Length(1), // location toggle
                Constraint::Min(0),   // spacer
            ])
            .split(content_area);

            let kn_focused =
                auth_focused && app.focus.auth_field == AuthField::KeyName;
            let kv_focused =
                auth_focused && app.focus.auth_field == AuthField::KeyValue;
            let loc_focused =
                auth_focused && app.focus.auth_field == AuthField::KeyLocation;

            let kn_style = if kn_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Cyan)
            };
            frame.render_widget(
                Paragraph::new("Key:").style(kn_style),
                field_chunks[0],
            );
            frame.render_widget(&app.request.auth_key_name_editor, field_chunks[1]);

            let kv_style = if kv_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Cyan)
            };
            frame.render_widget(
                Paragraph::new("Value:").style(kv_style),
                field_chunks[2],
            );
            frame.render_widget(&app.request.auth_key_value_editor, field_chunks[3]);

            let loc_label = match app.request.api_key_location {
                ApiKeyLocation::Header => "Add to: [Header]",
                ApiKeyLocation::QueryParam => "Add to: [Query Param]",
            };
            let loc_style = if loc_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            frame.render_widget(
                Paragraph::new(loc_label).style(loc_style),
                field_chunks[4],
            );
        }
    }
}

fn render_response_panel(frame: &mut Frame, app: &mut App, area: Rect) {
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

    let response_layout = ResponseLayout::new(inner_area);
    render_response_tab_bar(frame, app, response_layout.tab_area);
    frame.render_widget(Paragraph::new(""), response_layout.spacer_area);

    let editing_response =
        app.app_mode == AppMode::Editing && app.focus.panel == Panel::Response;
    let response_tab = app.response_tab;
    let response_scroll = app.response_scroll;
    match &app.response {
        ResponseStatus::Empty => {
            let hint = Paragraph::new("Press Ctrl+R to send request")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(hint, response_layout.content_area);
        }
        ResponseStatus::Loading => {
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame_idx = (app.loading_tick as usize / 4) % spinner_frames.len();
            let loading = Paragraph::new(format!("{} Sending request...", spinner_frames[frame_idx]))
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(loading, response_layout.content_area);
        }
        ResponseStatus::Error(msg) => {
            let error_lines = vec![Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red)),
                Span::raw(msg.as_str()),
            ])];
            let error_text = Paragraph::new(error_lines)
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            frame.render_widget(error_text, response_layout.content_area);
        }
        ResponseStatus::Cancelled => {
            let hint = Paragraph::new("⊘ Request cancelled")
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(hint, response_layout.content_area);
        }
        ResponseStatus::Success(data) => {
            match response_tab {
                ResponseTab::Body => {
                    let (response_editor, cache) =
                        (&app.response_editor, &mut app.response_body_cache);
                    render_response_body(
                        frame,
                        response_editor,
                        cache,
                        data,
                        response_layout.content_area,
                        response_scroll,
                        editing_response,
                    );
                }
                ResponseTab::Headers => {
                    let (response_headers_editor, cache) =
                        (&app.response_headers_editor, &mut app.response_headers_cache);
                    render_response_headers(
                        frame,
                        response_headers_editor,
                        cache,
                        response_layout.content_area,
                        response_scroll,
                        editing_response,
                    );
                }
            }
        }
    }
}

fn render_response_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_style) = response_status_text(app);
    let active_color = if app.focus.panel == Panel::Response {
        Color::Green
    } else {
        Color::White
    };
    let active_style = Style::default()
        .fg(active_color)
        .add_modifier(Modifier::UNDERLINED);
    let inactive_style = Style::default().fg(Color::DarkGray);
    let tabs_line = Line::from(vec![
        Span::styled(
            "Body",
            if app.response_tab == ResponseTab::Body {
                active_style
            } else {
                inactive_style
            },
        ),
        Span::styled(" | ", inactive_style),
        Span::styled(
            "Headers",
            if app.response_tab == ResponseTab::Headers {
                active_style
            } else {
                inactive_style
            },
        ),
    ]);

    let tabs_widget = Paragraph::new(tabs_line);
    frame.render_widget(tabs_widget, area);

    let status_widget =
        Paragraph::new(Line::from(Span::styled(status_text, status_style)))
            .alignment(Alignment::Right);
    frame.render_widget(status_widget, area);
}

fn response_status_text(app: &App) -> (String, Style) {
    match &app.response {
        ResponseStatus::Empty => (
            "Idle".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        ResponseStatus::Loading => (
            "Sending request...".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        ResponseStatus::Error(_) => ("Error".to_string(), Style::default().fg(Color::Red)),
        ResponseStatus::Cancelled => (
            "Cancelled".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        ResponseStatus::Success(data) => (
            format!("{} {} ({}ms)", data.status, data.status_text, data.duration_ms),
            Style::default().fg(status_color(data.status)),
        ),
    }
}

fn status_color(status: u16) -> Color {
    if status >= 200 && status < 300 {
        Color::Green
    } else if status >= 400 {
        Color::Red
    } else {
        Color::Yellow
    }
}

fn render_response_body(
    frame: &mut Frame,
    response_editor: &TextArea<'static>,
    cache: &mut ResponseBodyRenderCache,
    data: &crate::app::ResponseData,
    area: Rect,
    scroll_offset: u16,
    editing: bool,
) {
    if cache.dirty {
        let editor_lines = response_editor.lines();
        cache.body_text = editor_lines.join("\n");
        cache.is_json = is_json_response(&data.headers, &cache.body_text);
        cache.lines = if cache.is_json {
            colorize_json(&cache.body_text)
        } else {
            editor_lines
                .iter()
                .map(|l| Line::from(l.clone()))
                .collect()
        };
        cache.generation = cache.generation.wrapping_add(1);
        cache.dirty = false;
        cache.wrap_cache.generation = 0;
    }
    let cursor = if editing {
        Some(response_editor.cursor())
    } else {
        None
    };
    let selection = if editing {
        response_editor.selection_range()
    } else {
        None
    };
    render_wrapped_response_cached(
        frame,
        area,
        &cache.lines,
        &mut cache.wrap_cache,
        cache.generation,
        cursor,
        selection,
        scroll_offset,
        editing,
    );
}

fn render_response_headers(
    frame: &mut Frame,
    response_headers_editor: &TextArea<'static>,
    cache: &mut ResponseHeadersRenderCache,
    area: Rect,
    scroll_offset: u16,
    editing: bool,
) {
    if cache.dirty {
        let header_lines = response_headers_editor.lines();
        cache.lines = colorize_headers(header_lines);
        cache.generation = cache.generation.wrapping_add(1);
        cache.dirty = false;
        cache.wrap_cache.generation = 0;
    }
    let cursor = if editing {
        Some(response_headers_editor.cursor())
    } else {
        None
    };
    let selection = if editing {
        response_headers_editor.selection_range()
    } else {
        None
    };
    render_wrapped_response_cached(
        frame,
        area,
        &cache.lines,
        &mut cache.wrap_cache,
        cache.generation,
        cursor,
        selection,
        scroll_offset,
        editing,
    );
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
    let mut current_token = String::new();
    let mut stack: Vec<char> = Vec::new();
    let mut expecting_key = false;
    let mut current_string_is_key = false;

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_string => {
                in_string = true;
                current_string_is_key = expecting_key && matches!(stack.last(), Some('{'));
                current_token.push(c);
            }
            '"' if in_string => {
                current_token.push(c);
                let color = if current_string_is_key {
                    Color::Cyan
                } else {
                    Color::Green
                };
                current_spans.push(Span::styled(
                    std::mem::take(&mut current_token),
                    Style::default().fg(color),
                ));
                in_string = false;
                current_string_is_key = false;
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
            '{' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
                stack.push('{');
                expecting_key = true;
            }
            '}' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
                if stack.last() == Some(&'{') {
                    stack.pop();
                }
                expecting_key = false;
            }
            '[' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
                stack.push('[');
                expecting_key = false;
            }
            ']' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
                if stack.last() == Some(&'[') {
                    stack.pop();
                }
                expecting_key = false;
            }
            ':' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
                expecting_key = false;
            }
            ',' => {
                if !current_token.is_empty() {
                    let span = colorize_token(&current_token);
                    current_spans.push(span);
                    current_token.clear();
                }
                current_spans.push(Span::raw(c.to_string()));
                expecting_key = matches!(stack.last(), Some('{'));
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
    if token.trim().is_empty() {
        Span::raw(token.to_string())
    } else {
        Span::styled(token.to_string(), Style::default().fg(Color::Green))
    }
}

fn colorize_headers(lines: &[String]) -> Vec<Line<'static>> {
    lines
        .iter()
        .map(|line| {
            if let Some((key, rest)) = line.split_once(':') {
                Line::from(vec![
                    Span::styled(format!("{}:", key), Style::default().fg(Color::Cyan)),
                    Span::raw(rest.to_string()),
                ])
            } else {
                Line::from(line.clone())
            }
        })
        .collect()
}

fn render_wrapped_response_cached(
    frame: &mut Frame,
    area: Rect,
    lines: &[Line<'static>],
    cache: &mut WrapCache,
    lines_generation: u64,
    cursor: Option<(usize, usize)>,
    selection: Option<((usize, usize), (usize, usize))>,
    scroll_offset: u16,
    show_cursor: bool,
) {
    let _guard = perf::scope("render_wrapped_response_cached");
    if area.height == 0 || area.width == 0 {
        return;
    }

    let width = area.width as usize;
    let needs_rewrap = cache.width != width
        || cache.generation != lines_generation
        || cache.cursor != cursor
        || cache.selection != selection;
    if needs_rewrap {
        let (wrapped_lines, cursor_pos) =
            wrap_lines_with_cursor(lines, width, cursor, selection);
        cache.width = width;
        cache.generation = lines_generation;
        cache.cursor = cursor;
        cache.selection = selection;
        cache.wrapped_lines = wrapped_lines;
        cache.cursor_pos = cursor_pos;
    }

    let height = area.height as usize;
    let mut scroll_y = scroll_offset as usize;
    if show_cursor {
        if let Some((_, cursor_y)) = cache.cursor_pos {
            if cursor_y >= scroll_y + height {
                scroll_y = cursor_y.saturating_sub(height.saturating_sub(1));
            } else if cursor_y < scroll_y {
                scroll_y = cursor_y;
            }
        }
    }

    let visible_lines: Vec<Line<'static>> = cache
        .wrapped_lines
        .iter()
        .skip(scroll_y)
        .take(height)
        .cloned()
        .collect();

    let body_widget = Paragraph::new(visible_lines);
    frame.render_widget(body_widget, area);

    if show_cursor {
        if let Some((cursor_x, cursor_y)) = cache.cursor_pos {
            if cursor_y >= scroll_y {
                let rel_y = cursor_y - scroll_y;
                if rel_y < height {
                    let max_x = area.width.saturating_sub(1) as usize;
                    let clamped_x = cursor_x.min(max_x);
                    let x = area.x.saturating_add(clamped_x as u16);
                    let y = area.y.saturating_add(rel_y as u16);
                    frame.set_cursor_position((x, y));
                }
            }
        }
    }
}

fn wrap_lines_with_cursor(
    lines: &[Line<'static>],
    width: usize,
    cursor: Option<(usize, usize)>,
    selection: Option<((usize, usize), (usize, usize))>,
) -> (Vec<Line<'static>>, Option<(usize, usize)>) {
    let _guard = perf::scope("wrap_lines_with_cursor");
    let width = width.max(1);
    let mut wrapped_lines = Vec::new();
    let mut cursor_pos: Option<(usize, usize)> = None;

    for (row, line) in lines.iter().enumerate() {
        let line_len = line_char_len(&line);
        let selection_range = selection_range_for_row(selection, row, line_len);
        let cursor_col = cursor.and_then(|(r, c)| if r == row { Some(c) } else { None });
        let (parts, line_cursor) =
            wrap_line_spans_with_cursor(&line.spans, width, cursor_col, selection_range);
        if let Some((line_idx, col)) = line_cursor {
            cursor_pos = Some((col, wrapped_lines.len() + line_idx));
        }
        for spans in parts {
            wrapped_lines.push(Line::from(spans));
        }
    }

    if wrapped_lines.is_empty() {
        wrapped_lines.push(Line::from(""));
    }

    (wrapped_lines, cursor_pos)
}

fn line_char_len(line: &Line<'static>) -> usize {
    line.spans
        .iter()
        .map(|span| span.content.chars().count())
        .sum()
}

fn selection_range_for_row(
    selection: Option<((usize, usize), (usize, usize))>,
    row: usize,
    line_len: usize,
) -> Option<(usize, usize)> {
    let ((start_row, start_col), (end_row, end_col)) = selection?;
    if row < start_row || row > end_row {
        return None;
    }
    let start = if row == start_row { start_col } else { 0 };
    let end = if row == end_row { end_col } else { line_len };
    if start >= end {
        None
    } else {
        Some((start, end))
    }
}

fn wrap_line_spans_with_cursor(
    spans: &[Span<'static>],
    width: usize,
    cursor_col: Option<usize>,
    selection: Option<(usize, usize)>,
) -> (Vec<Vec<Span<'static>>>, Option<(usize, usize)>) {
    let width = width.max(1);
    let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut current_width = 0usize;
    let mut cursor_pos: Option<(usize, usize)> = None;
    let mut char_index = 0usize;

    for span in spans {
        for ch in span.content.chars() {
            if cursor_col == Some(char_index) && cursor_pos.is_none() {
                cursor_pos = Some((lines.len(), current_width));
            }

            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + ch_width > width && current_width > 0 {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }

            let mut style = span.style;
            if let Some((sel_start, sel_end)) = selection {
                if char_index >= sel_start && char_index < sel_end {
                    style = style.bg(Color::LightBlue);
                }
            }

            push_span_char(&mut current, style, ch);
            current_width += ch_width;
            char_index += 1;
        }
    }

    if cursor_col == Some(char_index) && cursor_pos.is_none() {
        cursor_pos = Some((lines.len(), current_width));
    }

    if current.is_empty() && lines.is_empty() {
        lines.push(Vec::new());
    } else {
        lines.push(current);
    }

    (lines, cursor_pos)
}

fn push_span_char(spans: &mut Vec<Span<'static>>, style: Style, ch: char) {
    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.content.to_mut().push(ch);
            return;
        }
    }
    spans.push(Span::styled(ch.to_string(), style));
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (mode_text, mode_style) = match app.app_mode {
        AppMode::Navigation => (
            " NAVIGATION ",
            Style::default()
                .fg(Color::Red)
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
        AppMode::Sidebar => (
            " SIDEBAR ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    };

    let panel_info = match app.focus.panel {
        Panel::Sidebar => "Sidebar".to_string(),
        Panel::Request => {
            let field = match app.focus.request_field {
                RequestField::Method => "Method",
                RequestField::Url => "URL",
                RequestField::Send => "Send",
                RequestField::Headers => "Headers",
                RequestField::Auth => "Auth",
                RequestField::Body => "Body",
            };
            format!("Request > {}", field)
        }
        Panel::Response => format!("Response > {}", app.response_tab.label()),
    };

    let hints = if app.focus.panel == Panel::Sidebar {
        if matches!(app.app_mode, AppMode::Sidebar) {
            "j/k:move  a:add  r:rename  d:del  m:move  /:search  Enter:open  Esc:exit"
        } else {
            "Enter/i:edit  hjkl:nav  Ctrl+p:projects  Ctrl+e:toggle"
        }
    } else {
        match app.app_mode {
            AppMode::Navigation => {
                "hjkl:nav  e:sidebar  Enter:edit  i:insert  Ctrl+r:send  Ctrl+s:save  Ctrl+e:toggle  ?:help  q:quit"
            }
            AppMode::Editing => match app.vim.mode {
                VimMode::Normal => {
                    "hjkl:move  w/b/e:word  i/a:insert  v:visual  d/c/y:op  Cmd/Ctrl+C/V:clip  Esc:exit"
                }
                VimMode::Insert => {
                    "type text  Cmd/Ctrl+V:paste  Cmd/Ctrl+C:copy  Enter:send(URL)  Esc:normal"
                }
                VimMode::Visual => {
                    "motion:select  d:delete  y:yank  c:change  Cmd/Ctrl+C/V:clip  Esc:cancel"
                }
                VimMode::Operator(_) => "motion:complete  Esc:cancel",
            },
            AppMode::Sidebar => "j/k:move  a:add  r:rename  d:del  m:move  /:search  Enter:open  Esc:exit",
        }
    };

    let mut status_spans = vec![
        Span::styled(mode_text, mode_style),
        Span::raw("  "),
        Span::raw(panel_info),
        Span::raw("  │  "),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
    ];

    if let Some(msg) = app.clipboard_toast_message() {
        status_spans.push(Span::raw("  │  "));
        status_spans.push(Span::styled(
            format!("Clipboard: {msg}"),
            Style::default().fg(Color::Yellow),
        ));
    }

    let status_line = Line::from(status_spans);

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
        Line::from("  h/j/k/l     Move focus across UI"),
        Line::from("  Arrow keys  Same as h/j/k/l"),
        Line::from("  e           Focus sidebar"),
        Line::from("  Enter       Activate field (vim normal mode)"),
        Line::from("  i           Enter field (vim insert mode)"),
        Line::from("  Ctrl+r      Send request"),
        Line::from("  Ctrl+e      Toggle sidebar (enter sidebar when opening)"),
        Line::from("  Ctrl+p      Project switcher"),
        Line::from("  Ctrl+s      Save request"),
        Line::from("  q / Esc     Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Sidebar",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Enter / i   Edit sidebar"),
        Line::from("  Esc         Return to navigation"),
        Line::from("  j/k or ↑/↓  Move selection"),
        Line::from("  h           Collapse / parent"),
        Line::from("  l / Enter   Toggle folder / open request"),
        Line::from("  a           Add request or folder"),
        Line::from("  r           Rename"),
        Line::from("  d           Delete"),
        Line::from("  D           Duplicate"),
        Line::from("  m           Move"),
        Line::from("  c           Copy path"),
        Line::from("  /           Search"),
        Line::from("  [ / ]       Outdent / indent"),
        Line::from("  Shift+h/l   Collapse / expand all"),
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
        Line::from("  clipboard   y/d/c/x/D/C -> system; p from system"),
        Line::from("  Cmd/Ctrl+C  Copy selection to system clipboard"),
        Line::from("  Cmd/Ctrl+V  Paste from system clipboard"),
        Line::from("  u / Ctrl+r  Undo / redo"),
        Line::from("  Enter       Send request (URL field only)"),
        Line::from("  Esc         Exit to navigation mode"),
    ];

    let help_paragraph = Paragraph::new(help_text);
    frame.render_widget(help_paragraph, help_inner);
}
