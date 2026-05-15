use crate::component::{
    Action, AttributeComponent, BookmarkComponent, Component, FileInfoComponent, GrepComponent,
};
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub enum SidePanel {
    Bookmark(BookmarkComponent),
    Grep(GrepComponent),
    FileInfo(FileInfoComponent),
    Attribute(AttributeComponent),
}

impl Component for SidePanel {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match self {
            SidePanel::Attribute(c) => c.handle_event(event),
            SidePanel::FileInfo(c) => c.handle_event(event),
            SidePanel::Bookmark(c) => c.handle_event(event),
            SidePanel::Grep(c) => c.handle_event(event),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match self {
            SidePanel::Attribute(c) => c.render(frame, area),
            SidePanel::FileInfo(c) => c.render(frame, area),
            SidePanel::Bookmark(c) => c.render(frame, area),
            SidePanel::Grep(c) => c.render(frame, area),
        }
    }

    fn tick(&mut self) {
        match self {
            SidePanel::Attribute(c) => c.tick(),
            SidePanel::FileInfo(c) => c.tick(),
            SidePanel::Bookmark(c) => c.tick(),
            SidePanel::Grep(c) => c.tick(),
        }
    }
}
