use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Mode, Panel};

/// Wrap text into lines that fit within max_width
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let current_len = current_line.chars().count();
        
        if current_len == 0 {
            // First word on line
            if word_len > max_width {
                // Word too long, split it
                let mut chars = word.chars();
                while chars.clone().count() > 0 {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if chunk.is_empty() { break; }
                    lines.push(chunk);
                }
            } else {
                current_line = word.to_string();
            }
        } else if current_len + 1 + word_len <= max_width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            // Word doesn't fit, start new line
            lines.push(current_line);
            if word_len > max_width {
                let mut chars = word.chars();
                while chars.clone().count() > 0 {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if chunk.is_empty() { break; }
                    lines.push(chunk);
                }
                current_line = String::new();
            } else {
                current_line = word.to_string();
            }
        }
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    if lines.is_empty() {
        lines.push(String::new());
    }
    
    lines
}

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
    use ratatui::text::{Line, Span};
    
    let is_focused = app.panel == Panel::Chats;
    let border_color = if is_focused {
        Color::Rgb(70, 130, 180)
    } else {
        Color::Rgb(50, 50, 60)
    };

    // Max bubble width = 60% of panel width
    let panel_width = area.width.saturating_sub(4) as usize;
    let max_bubble_width = (panel_width * 60) / 100;

    let messages = app.current_messages();
    let mut items: Vec<ListItem> = Vec::new();
    
    for msg in messages.iter() {
        let text = msg.text.trim();
        
        // Skip empty messages
        if text.is_empty() {
            continue;
        }
        
        // Wrap text into lines that fit the bubble
        let wrap_width = max_bubble_width.saturating_sub(4);
        let wrapped_lines = wrap_text(text, wrap_width);
        
        if msg.outgoing {
            // Outgoing: right-aligned green text
            let style = Style::default().fg(Color::Rgb(100, 200, 100));
            let prefix_style = Style::default().fg(Color::Rgb(60, 140, 60));
            
            for (i, line_text) in wrapped_lines.iter().enumerate() {
                let prefix = if i == 0 { "▸ " } else { "  " };
                let content = format!("{}{}", prefix, line_text);
                let padding = panel_width.saturating_sub(content.chars().count());
                
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(" ".repeat(padding)),
                    Span::styled(prefix, prefix_style),
                    Span::styled(line_text.clone(), style),
                ])));
            }
            // Blank line after message
            items.push(ListItem::new(Line::from("")));
        } else {
            // Incoming: sender name then message
            let sender_display: String = msg.sender.chars().take(12).collect();
            
            let sender_style = Style::default()
                .fg(Color::Rgb(100, 180, 255))
                .add_modifier(Modifier::BOLD);
            let text_style = Style::default().fg(Color::Rgb(200, 200, 200));
            
            // First line: sender + text
            if let Some(first_line) = wrapped_lines.first() {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!(" {}: ", sender_display), sender_style),
                    Span::styled(first_line.clone(), text_style),
                ])));
            }
            
            // Continuation lines with indent
            let indent_len = sender_display.chars().count() + 4;
            for line_text in wrapped_lines.iter().skip(1) {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(" ".repeat(indent_len)),
                    Span::styled(line_text.clone(), text_style),
                ])));
            }
            // Blank line after message
            items.push(ListItem::new(Line::from("")));
        }
    }

    // Get selected chat name for title (include loading status if present)
    let title = if let Some(status) = &app.loading_status {
        format!(" {} ", status)
    } else if let Some(chat) = app.chats.get(app.selected_chat) {
        format!(" {} ", chat.name)
    } else {
        " chats ".to_string()
    };

    // Apply scroll offset - skip items based on scroll position
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = app.scroll_offset.min(items.len().saturating_sub(1));
    let end = (scroll + visible_height).min(items.len());
    let visible_items: Vec<ListItem> = items.into_iter().skip(scroll).take(end - scroll).collect();

    let list = List::new(visible_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(title),
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
