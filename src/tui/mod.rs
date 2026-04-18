pub mod events;
pub mod state;
pub mod ui;

pub fn run(initial: crate::settings::Settings) -> Result<Option<crate::settings::Settings>, String> {
    let _ = initial;
    Err("non implémenté".to_string())
}
