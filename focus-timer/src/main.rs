pub mod app;
pub mod event;
pub mod tui;
pub mod ui;
pub mod timer;

use crate::app::App;
use crate::tui::Tui;
use std::io;

fn main() -> io::Result<()> {
    let mut tui = Tui::new()?;
    tui.enter()?;

    let mut app = App::new();
    let result = app.run(&mut tui);

    tui.exit()?;
    result
}
