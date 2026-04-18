use super::state::App;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('s') => {
            app.should_save = true;
            app.should_quit = true;
        }
        KeyCode::Tab | KeyCode::BackTab => app.next_panel(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_current_option(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::{App, Panel};
    use super::*;
    use crate::settings::Settings;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn q_quits_without_save() {
        let mut app = App::new(Settings::default());
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
        assert!(!app.should_save);
    }

    #[test]
    fn s_saves_and_quits() {
        let mut app = App::new(Settings::default());
        handle_key(&mut app, key(KeyCode::Char('s')));
        assert!(app.should_quit);
        assert!(app.should_save);
    }

    #[test]
    fn space_toggles_in_options_panel() {
        let mut app = App::new(Settings::default());
        app.focus = Panel::Options;
        app.option_idx = 3;
        assert!(app.settings.segments.git);
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(!app.settings.segments.git);
    }

    #[test]
    fn tab_switches_panel() {
        let mut app = App::new(Settings::default());
        handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.focus, Panel::Options);
    }
}
