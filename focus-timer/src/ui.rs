use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};

pub fn render(app: &App, frame: &mut Frame) {
    // Basic layout: Main (80%) and Help (20%)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(80),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(frame.area());

    // Sub-layout for the main area: Timer (70%) and Inputs (30%)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ]
            .as_ref(),
        )
        .split(chunks[0]);

    // Timer Block
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title("Focus Timer");

    let total_seconds = app.timer.remaining.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let timer_text = format!("{:02}:{:02}", minutes, seconds);

    let status_text = format!("Status: {:?}", app.timer.state);
    
    let combined_text = format!("{}\n\n{}", timer_text, status_text);

    let timer_paragraph = Paragraph::new(combined_text)
        .block(title_block)
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(timer_paragraph, main_chunks[0]);

    // Inputs Block
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Work
                Constraint::Length(3), // Break
                Constraint::Length(4), // Settings
            ]
            .as_ref(),
        )
        .split(main_chunks[1]);

    let work_style = match app.input_mode {
        InputMode::EditingWork => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };
    let work_input = Paragraph::new(app.work_input.as_str())
        .style(work_style)
        .block(Block::default().borders(Borders::ALL).title("Work (min)"));
    frame.render_widget(work_input, input_chunks[0]);

    let break_style = match app.input_mode {
        InputMode::EditingBreak => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };
    let break_input = Paragraph::new(app.break_input.as_str())
        .style(break_style)
        .block(Block::default().borders(Borders::ALL).title("Break (min)"));
    frame.render_widget(break_input, input_chunks[1]);

    // Status Block (Sound/Notifications)
    let settings_text = format!(
        "Sound: {}\nNotifications: {}",
        if app.sound_enabled { "ON" } else { "OFF" },
        if app.notifications_enabled { "ON" } else { "OFF" }
    );
    let settings_paragraph = Paragraph::new(settings_text)
        .block(Block::default().borders(Borders::ALL).title("Settings"));
    frame.render_widget(settings_paragraph, input_chunks[2]);
    
    // Help block
    let help_text = match app.input_mode {
        InputMode::Normal => "Press 'q' to quit, 'Space' to pause/resume, 'w' to edit work, 'b' to edit break, 's' toggle sound, 'n' toggle notifications",
        _ => "Press 'Enter' to save, 'Esc' to cancel",
    };
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    
    frame.render_widget(help_paragraph, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::app::App;

    #[test]
    fn test_ui_render_does_not_panic() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new();

        terminal.draw(|f| {
            render(&app, f);
        }).unwrap();
    }

    #[test]
    fn test_ui_render_shows_notifications_status() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.sound_enabled = true;
        app.notifications_enabled = false;

        terminal.draw(|f| {
            render(&app, f);
        }).unwrap();
        
        let buffer = terminal.backend().buffer();
        // Check if status is in the buffer. 
        // This is a bit brittle, but confirms we are rendering something.
        let content = format!("{:?}", buffer);
        assert!(content.contains("Sound: ON"));
        assert!(content.contains("Notifications: OFF"));
    }
}
