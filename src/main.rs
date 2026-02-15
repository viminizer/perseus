mod app;
mod clipboard;
mod config;
mod http;
mod perf;
mod storage;
mod ui;
mod vim;

use anyhow::Result;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new()?;
    app.run().await
}
