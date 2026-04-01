use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use super::{PopupState, SuggestionItem};

pub fn draw(f: &mut Frame, state: &PopupState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(f.area());

    // Search input
    let input = Paragraph::new(state.query.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Search "),
        );
    f.render_widget(input, chunks[0]);

    // Set cursor position after the query text
    f.set_cursor_position((
        chunks[0].x + state.query.len() as u16 + 1,
        chunks[0].y + 1,
    ));

    // Suggestion list
    let items: Vec<ListItem> = state
        .filtered
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let (tag, tag_color) = match item {
                SuggestionItem::App(_) => ("App", Color::Green),
                SuggestionItem::Project(_) => ("Project", Color::Magenta),
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" [{tag}] "),
                    Style::default().fg(tag_color).bold(),
                ),
                Span::styled(
                    item.name().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]);

            let style = if i == state.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let count = state.filtered.len();
    let total = state.all_items.len();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" Results ({count}/{total}) ")),
    );
    f.render_widget(list, chunks[1]);
}
