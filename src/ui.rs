use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{Mode, Model, Screen, PAGE_SIZE};

pub fn view(model: &mut Model, f: &mut Frame) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    // Left 1/3 indexes, right 2/3 documents — both always visible.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(rows[0]);

    index_list(model, f, cols[0]);
    documents(model, f, cols[1]);

    f.render_widget(Paragraph::new(footer_text(model)), rows[1]);
}

fn selected_style() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// Border style: highlighted when this pane has focus.
fn pane_border(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

fn index_list(model: &Model, f: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = model
        .indexes
        .iter()
        .map(|r| ListItem::new(format!("{}  ({})", r.uid, r.count)))
        .collect();

    let title = if model.mode == Mode::CreateInput {
        format!("New index: {}_", model.input)
    } else {
        "Indexes".to_string()
    };

    let focused = model.screen == Screen::IndexList;
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(pane_border(focused))
                .title(title),
        )
        .highlight_style(selected_style());

    let mut state = ListState::default();
    if !model.indexes.is_empty() {
        state.select(Some(model.index_sel));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn documents(model: &Model, f: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = model
        .docs
        .iter()
        .map(|d| ListItem::new(Line::from(d.as_str())))
        .collect();

    let pages = model.total_hits.div_ceil(PAGE_SIZE).max(1);
    let title = if model.mode == Mode::SearchInput {
        format!("/{}_", model.input)
    } else if model.current_index.is_empty() {
        "Documents  [Enter] an index to browse".to_string()
    } else {
        format!(
            "{}  [{} hits, page {}/{}]",
            model.current_index,
            model.total_hits,
            model.page + 1,
            pages,
        )
    };

    let focused = model.screen == Screen::Documents;
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(pane_border(focused))
                .title(title),
        )
        .highlight_style(selected_style());

    let mut state = ListState::default();
    if !model.docs.is_empty() {
        state.select(Some(model.doc_sel));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn footer_text(model: &Model) -> String {
    let keys = match model.screen {
        Screen::IndexList => "[c]reate [Enter]open [r]efresh [q]uit",
        Screen::Documents => "[/]search [n]ext [p]rev [Esc]back [q]uit",
    };
    format!(" {}  |  {keys}", model.status)
}
