use crate::component::{Action, Component};
use crate::fs::file_info::get_media_duration;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
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
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Sink,
    duration: Option<Duration>,
    is_playing: bool,
}

impl AudioPlayerComponent {
    pub fn new(path: &str, file_name: &str) -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("Failed to open audio output device")?;
        let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;

        let file = File::open(path).with_context(|| format!("Failed to open {path}"))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).with_context(|| format!("Failed to decode {path}"))?;

        let duration = get_media_duration(path).map(Duration::from_secs_f64);
        sink.append(source);

        let title = format!("Audio - {file_name}");

        Ok(Self {
            title,
            _stream: stream,
            _stream_handle: stream_handle,
            sink,
            duration,
            is_playing: true,
        })
    }

    fn toggle_play_pause(&mut self) {
        if self.is_playing {
            self.sink.pause();
            self.is_playing = false;
        } else {
            self.sink.play();
            self.is_playing = true;
        }
    }

    fn seek_forward(&self) {
        let current = self.sink.get_pos();
        let target = current + Duration::from_secs(SEEK_SECONDS);
        let target = match self.duration {
            Some(d) if target > d => d,
            _ => target,
        };
        let _ = self.sink.try_seek(target);
    }

    fn seek_backward(&self) {
        let current = self.sink.get_pos();
        let target = current.saturating_sub(Duration::from_secs(SEEK_SECONDS));
        let _ = self.sink.try_seek(target);
    }

    fn format_time(d: Duration) -> String {
        let total_secs = d.as_secs();
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }
}

impl Component for AudioPlayerComponent {
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
        let block = build_bordered_block(&self.title, BorderStyle::Active);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Time display: "再生秒数/全体秒数"
        let current = self.sink.get_pos();
        let current_str = Self::format_time(current);
        let time_line = match self.duration {
            Some(d) => format!("{current_str}/{}", Self::format_time(d)),
            None => current_str,
        };
        frame.render_widget(Paragraph::new(Line::from(time_line)), inner);
    }
}
