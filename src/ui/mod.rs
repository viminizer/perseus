mod layout;
mod widgets;

use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, _app: &App) {
    frame.render_widget(ratatui::widgets::Clear, frame.area());
}
