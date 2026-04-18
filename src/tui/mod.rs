pub mod events;
pub mod state;
pub mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use crate::settings::Settings;
use state::App;

pub fn run(initial: Settings) -> Result<Option<Settings>, String> {
    enable_raw_mode().map_err(|e| format!("raw mode: {}", e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| format!("alt screen: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("terminal init: {}", e))?;

    let mut app = App::new(initial);

    let result = main_loop(&mut terminal, &mut app);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).ok();
    terminal.show_cursor().ok();

    result?;

    if app.should_save {
        Ok(Some(app.settings))
    } else {
        Ok(None)
    }
}

fn main_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        terminal.draw(|f| ui::draw(f, app)).map_err(|e| format!("draw: {}", e))?;

        if event::poll(Duration::from_millis(100)).map_err(|e| format!("poll: {}", e))? {
            if let Event::Key(key) = event::read().map_err(|e| format!("read: {}", e))? {
                events::handle_key(app, key);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
