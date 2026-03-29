mod app;
mod cmd;
mod config;
mod event;
mod fs;
mod state;
mod ui;

use anyhow::Result;
use app::App;
use config::Config;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(Config::default());
    app.init()?;
    let result = app.run(&mut terminal);

    ratatui::restore();
    result
}
