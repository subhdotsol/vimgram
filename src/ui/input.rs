use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, Mode};

/// Handle keyboard input based on current mode
pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<String> {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return None;
    }

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Insert => handle_insert_mode(app, key),
    }
}

/// Handle keys in normal mode (vim navigation)
fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Option<String> {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('h') | KeyCode::Left => app.switch_panel(),
        KeyCode::Char('l') | KeyCode::Right => app.switch_panel(),
        
        // Mode switching
        KeyCode::Char('i') => app.enter_insert(),
        
        // Reload current chat
        KeyCode::Char('r') => app.reload_requested = true,
        
        // Quit
        KeyCode::Char('q') => app.should_quit = true,
        
        // Jump to top/bottom
        KeyCode::Char('g') => app.selected_chat = 0,
        KeyCode::Char('G') => {
            app.selected_chat = app.chats.len().saturating_sub(1);
        }
        
        _ => {}
    }
    None
}

/// Handle keys in insert mode (typing)
fn handle_insert_mode(app: &mut App, key: KeyEvent) -> Option<String> {
    match key.code {
        // Exit insert mode
        KeyCode::Esc => {
            app.exit_insert();
        }
        
        // Send message
        KeyCode::Enter => {
            if !app.input.is_empty() {
                let message = app.input.clone();
                app.input.clear();
                return Some(message);
            }
        }
        
        // Delete character
        KeyCode::Backspace => {
            app.input.pop();
        }
        
        // Type character
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        
        _ => {}
    }
    None
}
