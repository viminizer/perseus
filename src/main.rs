mod app;
mod http;
mod storage;
mod ui;
mod vim;

use anyhow::Result;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.run().await
}
