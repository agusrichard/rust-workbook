use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame) {
    // Basic layout
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

    frame.render_widget(timer_paragraph, chunks[0]);
    
    // Help block
    let help_text = "Press 'q' to quit, 'Space' to pause/resume";
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
}
