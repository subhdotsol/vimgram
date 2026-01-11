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

    // Split into friends (left) and right side (chats + input)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Friends panel (full height)
            Constraint::Percentage(70), // Right side: Chats + Input
        ])
        .split(inner_area);

    // Split right side into chats and input box
    let right_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),      // Chats panel
            Constraint::Length(3),   // Input box (under chats only)
        ])
        .split(horizontal[1]);

    // Draw panels
    draw_friends_panel(frame, app, horizontal[0]);
    draw_chats_panel(frame, app, right_vertical[0]);
    draw_input_box(frame, app, right_vertical[1]);
    
    // Draw account picker overlay if in that mode
    if app.mode == Mode::AccountPicker {
        draw_account_picker(frame, app, frame.area());
    }
    
    // Draw find user overlay if in that mode
    if app.mode == Mode::FindUser {
        draw_find_user(frame, app, frame.area());
    }
}

/// Draw the friends/contacts list panel
fn draw_friends_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.panel == Panel::Friends;
    let is_search_mode = app.mode == Mode::Search;
    
    let border_color = if is_search_mode {
        Color::Rgb(255, 180, 50) // Orange/yellow when in search mode
    } else if is_focused {
        Color::Rgb(70, 130, 180) // Bright blue when focused
    } else {
        Color::Rgb(50, 50, 60) // Dim when not focused
    };

    // Determine which chats to display
    let (display_indices, highlight_idx): (Vec<usize>, usize) = if is_search_mode {
        (app.filtered_chat_indices.clone(), app.search_selected)
    } else {
        ((0..app.chats.len()).collect(), app.selected_chat)
    };

    let items: Vec<ListItem> = display_indices
        .iter()
        .enumerate()
        .filter_map(|(display_idx, &chat_idx)| {
            app.chats.get(chat_idx).map(|chat| {
                let is_selected = display_idx == highlight_idx;
                let style = if is_selected && (is_focused || is_search_mode) {
                    Style::default()
                        .fg(if is_search_mode { Color::Rgb(255, 180, 50) } else { Color::Rgb(70, 130, 180) })
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(180, 180, 180))
                };

                let prefix = if is_selected && (is_focused || is_search_mode) { "> " } else { "  " };
                let unread = if chat.unread > 0 {
                    format!(" ({})", chat.unread)
                } else {
                    String::new()
                };

                ListItem::new(format!("{}{}{}", prefix, chat.name, unread)).style(style)
            })
        })
        .collect();

    // Build title with search input if in search mode
    let title = if is_search_mode {
        format!(" /{}▏", app.search_input)
    } else {
        " friends ".to_string()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(title),
    );

    frame.render_widget(list, area);
}

/// Draw the messages/chats panel
fn draw_chats_panel(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::layout::Alignment;
    
    let is_focused = app.panel == Panel::Chats;
    let border_color = if is_focused {
        Color::Rgb(70, 130, 180)
    } else {
        Color::Rgb(50, 50, 60)
    };

    // Check if this is the Welcome chat (id=1) - show centered welcome box
    let is_welcome_chat = app.current_chat_id() == Some(1);
    
    if is_welcome_chat {
        // Draw centered welcome box
        draw_welcome_box(frame, area, border_color);
        return;
    }

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
            let sender_display: String = msg.sender.chars().take(20).collect();
            if sender_display.trim().is_empty() {
                // Don't force "Unknown", just leave it empty
                // sender_display = "Unknown".to_string(); 
            }
            
            let sender_style = Style::default()
                .fg(Color::Rgb(100, 180, 255))
                .add_modifier(Modifier::BOLD);
            let text_style = Style::default().fg(Color::Rgb(200, 200, 200));
            
            // First line: sender + text
            
            // Heuristic: If sender name matches the Chat Title (and it's not empty),
            // it's likely a DM where the header already says who it is.
            let mut current_chat_name = "Unknown";
            if let Some(c) = app.chats.get(app.selected_chat) {
                current_chat_name = &c.name;
            }

            if let Some(first_line) = wrapped_lines.first() {
                // Hide if explicitly "Unknown", empty, or matches chat title (DM)
                let should_hide_name = 
                    sender_display == "Unknown" || 
                    sender_display.trim().is_empty() ||
                    (sender_display == current_chat_name && current_chat_name != "Unknown");

                if should_hide_name {
                    // Hide sender name, just show text (padded to align with other lines if desirable, 
                    // or just flush left. Standard TUI chat usually aligns flush left if no name).
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("  "), // Left padding
                        Span::styled(first_line.clone(), text_style),
                    ])));
                } else {
                    // Show sender name
                    // Pad aggressively to 20 chars to wipe any "Unknown" ghosting or artifacts
                    // format!("{:<20}", s) pads right with spaces to length 20.
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("  "), // Left padding
                        Span::styled(format!("{:<20}", sender_display), sender_style),
                        Span::raw(": "), 
                        Span::styled(first_line.clone(), text_style),
                    ])));
                }
            }
            
// Continuation lines with indent
            let should_hide_name = 
                sender_display == "Unknown" || 
                sender_display.trim().is_empty() ||
                (sender_display == current_chat_name && current_chat_name != "Unknown");

            let indent_len = if should_hide_name {
                2 // Just the left padding
            } else {
                sender_display.chars().count() + 4 + 2 // Name + ": " + left padding
            };

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

    // Apply scroll offset - bottom aligned
    let visible_height = area.height.saturating_sub(2) as usize;
    let total_items = items.len();
    
    // Calculate range based on inverted scroll_offset (0 = bottom)
    let end_index = total_items.saturating_sub(app.scroll_offset);
    let start_index = end_index.saturating_sub(visible_height);
    
    // Get the slice of messages
    let mut visible_items: Vec<ListItem> = items.into_iter()
        .skip(start_index)
        .take(end_index - start_index)
        .collect();
        
    // If fewer items than height, pad with empty lines to force bottom alignment
    if visible_items.len() < visible_height {
        let padding = visible_height - visible_items.len();
        let mut padded_items = vec![ListItem::new(""); padding];
        padded_items.extend(visible_items);
        visible_items = padded_items;
    }

    let list = List::new(visible_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(title),
    );

    frame.render_widget(list, area);
}

/// Draw a centered welcome box with keybindings
fn draw_welcome_box(frame: &mut Frame, area: Rect, border_color: Color) {
    use ratatui::text::{Line, Span};
    use ratatui::layout::Alignment;
    
    // Outer block for the chat panel
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Welcome ");
    
    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);
    
    // Don't render inner box if area is too small
    if inner_area.width < 20 || inner_area.height < 10 {
        // Just show a simple message in the center
        let simple_msg = Paragraph::new("Press j/k to navigate, / to search")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Rgb(180, 180, 180)));
        frame.render_widget(simple_msg, inner_area);
        return;
    }
    
    // Calculate centered box dimensions - ensure minimum viable size
    let content_width = 50;
    let content_height = 18;  // 15 lines + 2 for borders + 1 buffer
    
    let box_width = content_width.min(inner_area.width.saturating_sub(2));
    let box_height = content_height.min(inner_area.height.saturating_sub(2));
    
    // Ensure we don't underflow
    if box_width < 10 || box_height < 5 {
        return;
    }
    
    let box_x = inner_area.x + (inner_area.width.saturating_sub(box_width)) / 2;
    let box_y = inner_area.y + (inner_area.height.saturating_sub(box_height)) / 2;
    
    let welcome_area = Rect::new(box_x, box_y, box_width, box_height);
    
    // Welcome content
    let welcome_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚡ Welcome to Vimgram! ⚡",
            Style::default().fg(Color::Rgb(100, 180, 255)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("NORMAL MODE", Style::default().fg(Color::Rgb(255, 180, 50)).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("j/k scroll  h/l panels  / search  : cmd", Style::default().fg(Color::Rgb(180, 180, 180)))),
        Line::from(Span::styled("i insert  q quit  D disconnect", Style::default().fg(Color::Rgb(180, 180, 180)))),
        Line::from(""),
        Line::from(Span::styled("COMMAND MODE", Style::default().fg(Color::Rgb(100, 200, 100)).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(":find @user  search any Telegram user", Style::default().fg(Color::Rgb(180, 180, 180)))),
        Line::from(""),
        Line::from(Span::styled("SEARCH MODE", Style::default().fg(Color::Rgb(255, 180, 50)).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("type filter  arrows nav  Enter jump", Style::default().fg(Color::Rgb(180, 180, 180)))),
        Line::from(""),
        Line::from(Span::styled("INSERT MODE", Style::default().fg(Color::Rgb(255, 180, 50)).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("type msg  Enter send  Esc cancel", Style::default().fg(Color::Rgb(180, 180, 180)))),
    ];
    
    let welcome_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(70, 130, 180)))
        .border_type(ratatui::widgets::BorderType::Rounded);
    
    let paragraph = Paragraph::new(welcome_lines)
        .block(welcome_block)
        .alignment(Alignment::Center);
    
    frame.render_widget(paragraph, welcome_area);
}

/// Draw the input box at the bottom
fn draw_input_box(frame: &mut Frame, app: &App, area: Rect) {
    let (title, style) = match app.mode {
        Mode::Insert => (
            " INSERT ",
            Style::default().fg(Color::Rgb(70, 130, 180)),
        ),
        Mode::Search => (
            " / search (↑↓ navigate, Enter select, Esc cancel) ",
            Style::default().fg(Color::Rgb(255, 180, 50)),
        ),
        Mode::AccountPicker => (
            " A switch accounts (↑↓ navigate, Enter select, Esc cancel) ",
            Style::default().fg(Color::Rgb(150, 100, 255)),
        ),
        Mode::Command => (
            " COMMAND ",
            Style::default().fg(Color::Rgb(100, 200, 100)),
        ),
        Mode::FindUser => (
            " FIND USER ",
            Style::default().fg(Color::Rgb(100, 200, 255)),
        ),
        Mode::Normal => (
            " type to send ",
            Style::default().fg(Color::Rgb(80, 80, 90)),
        ),
    };

    // Content to display in input box
    let content = match app.mode {
        Mode::Command => format!(":{}", app.command_input),
        _ => app.input.clone(),
    };

    let input = Paragraph::new(content.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(title),
        );

    frame.render_widget(input, area);

    // Show cursor in insert mode or command mode
    if app.mode == Mode::Insert {
        frame.set_cursor_position((
            area.x + app.input.len() as u16 + 1,
            area.y + 1,
        ));
    } else if app.mode == Mode::Command {
        // +2 for ": " prefix
        frame.set_cursor_position((
            area.x + app.command_input.len() as u16 + 2,
            area.y + 1,
        ));
    }
}

/// Draw the account picker overlay
fn draw_account_picker(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Clear;
    
    // Calculate overlay dimensions
    let box_width = 40.min(area.width.saturating_sub(10));
    let box_height = (app.account_names.len() as u16 + 4).min(area.height.saturating_sub(6));
    
    let box_x = (area.width.saturating_sub(box_width)) / 2;
    let box_y = (area.height.saturating_sub(box_height)) / 2;
    
    let overlay_area = Rect::new(box_x, box_y, box_width, box_height);
    
    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay_area);
    
    // Build account list items
    let mut items: Vec<ListItem> = app.account_names
        .iter()
        .enumerate()
        .map(|(i, (id, name))| {
            let is_selected = i == app.account_picker_selected;
            let is_current = *id == app.current_account_id;
            
            let prefix = if is_selected { "> " } else { "  " };
            let suffix = if is_current { " ✓" } else { "" };
            
            let style = if is_selected {
                Style::default().fg(Color::Rgb(150, 100, 255)).add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(Color::Rgb(100, 200, 100))
            } else {
                Style::default().fg(Color::Rgb(180, 180, 180))
            };
            
            ListItem::new(format!("{}{}{}", prefix, name, suffix)).style(style)
        })
        .collect();
    
    // Add "+ Add Account" option
    let add_selected = app.account_picker_selected == app.account_names.len();
    let add_style = if add_selected {
        Style::default().fg(Color::Rgb(100, 200, 100)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(100, 180, 100))
    };
    let add_prefix = if add_selected { "> " } else { "  " };
    items.push(ListItem::new(format!("{}+ Add Account", add_prefix)).style(add_style));
    
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(150, 100, 255)))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(" Switch Account "),
    );
    
    frame.render_widget(list, overlay_area);
}

/// Draw the find user overlay
fn draw_find_user(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Clear;
    use crate::app::FindResult;
    
    // Calculate overlay dimensions
    let box_width = 50.min(area.width.saturating_sub(10));
    let box_height = 7.min(area.height.saturating_sub(6));
    
    let box_x = (area.width.saturating_sub(box_width)) / 2;
    let box_y = (area.height.saturating_sub(box_height)) / 2;
    
    let overlay_area = Rect::new(box_x, box_y, box_width, box_height);
    
    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay_area);
    
    // Build content based on find result
    let lines: Vec<Line> = match &app.find_result {
        Some(FindResult::Searching) => vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("🔍 Searching for @{}...", app.find_input),
                Style::default().fg(Color::Rgb(100, 200, 255)),
            )),
        ],
        Some(FindResult::Found { name, .. }) => vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("✅ Found: {}", name),
                Style::default().fg(Color::Rgb(100, 200, 100)).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Press Enter to start chatting, Esc to cancel",
                Style::default().fg(Color::Rgb(180, 180, 180)),
            )),
        ],
        Some(FindResult::NotFound(username)) => vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("❌ User @{} not found", username),
                Style::default().fg(Color::Rgb(255, 100, 100)),
            )),
            Line::from(Span::styled(
                "Press Esc to close",
                Style::default().fg(Color::Rgb(180, 180, 180)),
            )),
        ],
        Some(FindResult::Error(msg)) => vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("⚠️ Error: {}", msg),
                Style::default().fg(Color::Rgb(255, 180, 50)),
            )),
            Line::from(Span::styled(
                "Press Esc to close",
                Style::default().fg(Color::Rgb(180, 180, 180)),
            )),
        ],
        None => vec![
            Line::from(Span::styled(
                "Type a username to search...",
                Style::default().fg(Color::Rgb(180, 180, 180)),
            )),
        ],
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(100, 200, 255)))
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(format!(" :find @{} ", app.find_input));
    
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    
    frame.render_widget(paragraph, overlay_area);
}
