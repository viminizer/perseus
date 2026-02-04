use anyhow::Result;

pub struct App {
    running: bool,
}

impl App {
    pub fn new() -> Self {
        Self { running: true }
    }

    pub async fn run(&mut self) -> Result<()> {
        while self.running {
            // Event loop will be implemented in Task 3
            break;
        }
        Ok(())
    }
}
