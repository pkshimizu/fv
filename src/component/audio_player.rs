use crate::component::{Action, Component};
use crate::fs::file_info::{format_duration, get_media_duration};
use crate::ui::widgets::{BorderState, Focus, build_bordered_block};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

const SEEK_SECONDS: u64 = 5;

pub struct AudioPlayerComponent {
    title: String,
    // sink must be declared before _stream/_stream_handle to be dropped first
    sink: Sink,
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    duration: Option<Duration>,
    duration_str: Option<String>,
}

impl AudioPlayerComponent {
    pub fn new(path: &str, file_name: &str) -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("Failed to open audio output device")?;
        let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;

        let file = File::open(path).with_context(|| format!("Failed to open {path}"))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).with_context(|| format!("Failed to decode {path}"))?;

        let duration = get_media_duration(path);
        let duration_str = duration.map(format_duration);
        sink.append(source);

        let title = format!("Audio - {file_name}");

        Ok(Self {
            title,
            sink,
            _stream: stream,
            _stream_handle: stream_handle,
            duration,
            duration_str,
        })
    }

    fn toggle_play_pause(&self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }

    fn seek_forward(&self) {
        let current = self.sink.get_pos();
        let target = current + Duration::from_secs(SEEK_SECONDS);
        let target = match self.duration {
            Some(d) if target > d => d,
            _ => target,
        };
        if let Err(e) = self.sink.try_seek(target) {
            tracing::warn!("Seek failed: {e}");
        }
    }

    fn seek_backward(&self) {
        let current = self.sink.get_pos();
        let target = current.saturating_sub(Duration::from_secs(SEEK_SECONDS));
        if let Err(e) = self.sink.try_seek(target) {
            tracing::warn!("Seek failed: {e}");
        }
    }
}

impl Component for AudioPlayerComponent {
    fn keymap(&self) -> &'static str {
        "Space: Play/Pause  ←→: Seek  v/Esc: Close"
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char(' ') => {
                self.toggle_play_pause();
                Ok(Action::None)
            }
            KeyCode::Right => {
                self.seek_forward();
                Ok(Action::None)
            }
            KeyCode::Left => {
                self.seek_backward();
                Ok(Action::None)
            }
            KeyCode::Char('v') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = build_bordered_block(&self.title, Focus::Focused, BorderState::Normal);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let current_str = format_duration(self.sink.get_pos());
        let time_line = match &self.duration_str {
            Some(d) => format!("{current_str}/{d}"),
            None => current_str,
        };
        frame.render_widget(Paragraph::new(Line::from(time_line)), inner);
    }
}
