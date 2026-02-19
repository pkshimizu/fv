use crate::state::AppState;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

pub fn render_main_view(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    let app_name = Paragraph::new(Text::raw(&state.config.app_name))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    let status_bar =
        Layout::horizontal([Constraint::Length(5), Constraint::Fill(1)]).split(vertical[2]);

    let path_label = Paragraph::new(Text::raw("Path:"))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Left);
    let path = Paragraph::new(Text::raw(state.filer.current_dir_path.to_str().unwrap()))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Left);

    frame.render_widget(path_label, status_bar[0]);
    frame.render_widget(path, status_bar[1]);

    frame.render_widget(app_name, vertical[0]);
}
