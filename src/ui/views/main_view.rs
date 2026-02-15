use crate::state::AppState;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

pub fn render_main_view(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(area);

    let app_name = Paragraph::new(Text::raw(&state.config.app_name))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    frame.render_widget(app_name, vertical[1]);
}
