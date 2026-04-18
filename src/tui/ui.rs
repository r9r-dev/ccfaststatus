use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::state::{App, Category, Panel, ALL_CATEGORIES, ALL_SEGMENTS};
use crate::input::ClaudeInput;
use crate::theme::ALL_THEMES;

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Min(30),
            Constraint::Percentage(40),
        ])
        .split(size);

    draw_categories(f, chunks[0], app);
    draw_options(f, chunks[1], app);
    draw_preview(f, chunks[2], app);
}

fn draw_categories(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = ALL_CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let label = match cat {
                Category::Segments => "Segments",
                Category::Theme => "Thème",
            };
            let mut line = Line::from(label.to_string());
            if i == app.category_idx {
                line = line.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            ListItem::new(line)
        })
        .collect();

    let title = if app.focus == Panel::Categories { "[Catégories]" } else { "Catégories" };
    let block = Block::default().title(title).borders(Borders::ALL);
    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_options(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = match app.current_category() {
        Category::Segments => ALL_SEGMENTS
            .iter()
            .enumerate()
            .map(|(i, def)| {
                let checked = (def.getter)(&app.settings.segments);
                let mark = if checked { "◉" } else { "◯" };
                let text = format!(" {} {}", mark, def.label);
                let mut line = Line::from(text);
                if app.focus == Panel::Options && i == app.option_idx {
                    line = line.style(Style::default().add_modifier(Modifier::REVERSED));
                }
                ListItem::new(line)
            })
            .collect(),
        Category::Theme => ALL_THEMES
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let checked = app.settings.theme == t.name;
                let mark = if checked { "◉" } else { "◯" };
                let text = format!(" {} {}", mark, t.name);
                let mut line = Line::from(text);
                if app.focus == Panel::Options && i == app.option_idx {
                    line = line.style(Style::default().add_modifier(Modifier::REVERSED));
                }
                ListItem::new(line)
            })
            .collect(),
    };

    let title = if app.focus == Panel::Options { "[Options]" } else { "Options" };
    let block = Block::default().title(title).borders(Borders::ALL);
    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
    let data: ClaudeInput = serde_json::from_str("{}").expect("{} valid");
    let inner_width = area.width.saturating_sub(2) as usize;
    let rendered_ansi = crate::render_with(data, inner_width, app.settings.clone());
    let plain = crate::term::strip_ansi(&rendered_ansi);

    let help = "\n\nTab: panneau  ↑↓: naviguer  Espace: toggle  s: sauver  q: quitter";
    let text = format!("{}\n{}", plain, help);
    let p = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .block(Block::default().title("Aperçu").borders(Borders::ALL));
    f.render_widget(p, area);
}
