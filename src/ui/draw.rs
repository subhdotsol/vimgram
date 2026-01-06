use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Mode, Panel};

/// Main UI drawing function
pub fn draw(frame: &mut Frame, app: &App) {
    // Main container with outer border
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(70, 130, 180))) // Steel blue
        .title(" Bifrost ");

    let inner_area = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    // Split into main area and input box
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),      // Main panels
            Constraint::Length(3),   // Input box
        ])
        .split(inner_area);

    // Split main area into friends (left) and chats (right)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Friends panel
            Constraint::Percentage(70), // Chats panel
        ])
        .split(vertical[0]);

    // Draw panels
    draw_friends_panel(frame, app, horizontal[0]);
    draw_chats_panel(frame, app, horizontal[1]);
    draw_input_box(frame, app, vertical[1]);
}

/// Draw the friends/contacts list panel
fn draw_friends_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.panel == Panel::Friends;
    let border_color = if is_focused {
        Color::Rgb(70, 130, 180) // Bright blue when focused
    } else {
        Color::Rgb(50, 50, 60) // Dim when not focused
    };

    let items: Vec<ListItem> = app
        .chats
        .iter()
        .enumerate()
        .map(|(i, chat)| {
            let style = if i == app.selected_chat && is_focused {
                Style::default()
                    .fg(Color::Rgb(70, 130, 180))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(180, 180, 180))
            };

            let prefix = if i == app.selected_chat && is_focused { "> " } else { "  " };
            let unread = if chat.unread > 0 {
                format!(" ({})", chat.unread)
            } else {
                String::new()
            };

            ListItem::new(format!("{}{}{}", prefix, chat.name, unread)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(" friends "),
    );

    frame.render_widget(list, area);
}

/// Draw the messages/chats panel
fn draw_chats_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.panel == Panel::Chats;
    let border_color = if is_focused {
        Color::Rgb(70, 130, 180)
    } else {
        Color::Rgb(50, 50, 60)
    };

    // Calculate inner width for right-alignment (account for borders)
    let inner_width = area.width.saturating_sub(4) as usize;

    let messages = app.current_messages();
    let items: Vec<ListItem> = messages
        .iter()
        .map(|msg| {
            if msg.outgoing {
                // Outgoing: right-aligned, green
                let text = format!("You: {}", msg.text);
                let padding = inner_width.saturating_sub(text.len());
                let padded = format!("{}{}", " ".repeat(padding), text);
                ListItem::new(padded).style(Style::default().fg(Color::Rgb(100, 200, 100)))
            } else {
                // Incoming: left-aligned with sender name, gray/white
                let sender_style = Style::default()
                    .fg(Color::Rgb(130, 180, 230))
                    .add_modifier(Modifier::BOLD);
                let msg_style = Style::default().fg(Color::Rgb(220, 220, 220));
                
                let line = ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("{}: ", msg.sender), sender_style),
                    ratatui::text::Span::styled(&msg.text, msg_style),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(" chats "),
    );

    frame.render_widget(list, area);
}

/// Draw the input box at the bottom
fn draw_input_box(frame: &mut Frame, app: &App, area: Rect) {
    let (title, style) = match app.mode {
        Mode::Insert => (
            " INSERT ",
            Style::default().fg(Color::Rgb(70, 130, 180)),
        ),
        Mode::Normal => (
            " type to send ",
            Style::default().fg(Color::Rgb(80, 80, 90)),
        ),
    };

    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(title),
        );

    frame.render_widget(input, area);

    // Show cursor in insert mode
    if app.mode == Mode::Insert {
        frame.set_cursor_position((
            area.x + app.input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}
