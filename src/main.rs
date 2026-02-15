mod app;
mod cmd;
mod config;
mod event;
mod fs;
mod state;
mod ui;

use app::App;
use config::Config;

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new(Config::default()).run(&mut terminal);
    ratatui::restore();
    result
}
