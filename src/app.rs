use std::io::stdout;
use std::panic;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::ui;

pub struct App {
    running: bool,
}

impl App {
    pub fn new() -> Self {
        Self { running: true }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.install_panic_hook();
        self.setup_terminal()?;

        let result = self.event_loop().await;

        self.restore_terminal()?;
        result
    }

    fn install_panic_hook(&self) {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = stdout().execute(LeaveAlternateScreen);
            original_hook(panic_info);
        }));
    }

    fn setup_terminal(&self) -> Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn restore_terminal(&self) -> Result<()> {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    async fn event_loop(&mut self) -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        while self.running {
            terminal.draw(|frame| {
                ui::render(frame, self);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        self.running = false;
                    }
                }
            }
        }

        Ok(())
    }
}
