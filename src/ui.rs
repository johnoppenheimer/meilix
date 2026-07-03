use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{Mode, Model, Screen, PAGE_SIZE};

pub fn view(model: &mut Model, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    match model.screen {
        Screen::IndexList => index_list(model, f, chunks[0]),
        Screen::Documents => documents(model, f, chunks[0]),
    }

    let footer = footer_text(model);
    f.render_widget(Paragraph::new(footer), chunks[1]);
}

fn selected_style() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

fn index_list(model: &Model, f: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = model
        .indexes
        .iter()
        .map(|r| ListItem::new(format!("{}  ({})", r.uid, r.count)))
        .collect();

    let title = if model.mode == Mode::CreateInput {
        format!("New index: {}_", model.input)
    } else {
        "Indexes  [c]reate  [Enter] open  [r]efresh  [q]uit".to_string()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(selected_style());

    let mut state = ListState::default();
    if !model.indexes.is_empty() {
        state.select(Some(model.index_sel));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn documents(model: &Model, f: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = model
        .docs
        .iter()
        .map(|d| ListItem::new(Line::from(d.as_str())))
        .collect();

    let pages = model.total_hits.div_ceil(PAGE_SIZE).max(1);
    let title = if model.mode == Mode::SearchInput {
        format!("/{}_", model.input)
    } else {
        format!(
            "{}  [{} hits, page {}/{}]  [/]search [n]ext [p]rev [Esc]back",
            model.current_index,
            model.total_hits,
            model.page + 1,
            pages,
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(selected_style());

    let mut state = ListState::default();
    if !model.docs.is_empty() {
        state.select(Some(model.doc_sel));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn footer_text(model: &Model) -> String {
    format!(" {}", model.status)
}
