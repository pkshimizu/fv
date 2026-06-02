use crate::component::{
    Action, AttributeComponent, AudioPlayerComponent, BookmarkComponent, Component,
    FileInfoComponent, GrepComponent, HelpComponent, ImagePreviewComponent, PreviewComponent,
    SettingsComponent, TreeComponent,
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
    Settings(SettingsComponent),
    Tree(TreeComponent),
    Preview(PreviewComponent),
    AudioPlayer(AudioPlayerComponent),
    ImagePreview(ImagePreviewComponent),
    Help(HelpComponent),
}

impl SidePanel {
    /// プレビュー系パネル（テキスト/画像/音声/メッセージ）かどうか。
    /// n/p によるファイル切り替え後の再生成対象を判定するのに使う。
    pub fn is_preview(&self) -> bool {
        matches!(
            self,
            SidePanel::Preview(_) | SidePanel::AudioPlayer(_) | SidePanel::ImagePreview(_)
        )
    }
}

impl Component for SidePanel {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match self {
            SidePanel::Attribute(c) => c.handle_event(event),
            SidePanel::FileInfo(c) => c.handle_event(event),
            SidePanel::Bookmark(c) => c.handle_event(event),
            SidePanel::Grep(c) => c.handle_event(event),
            SidePanel::Settings(c) => c.handle_event(event),
            SidePanel::Tree(c) => c.handle_event(event),
            SidePanel::Preview(c) => c.handle_event(event),
            SidePanel::AudioPlayer(c) => c.handle_event(event),
            SidePanel::ImagePreview(c) => c.handle_event(event),
            SidePanel::Help(c) => c.handle_event(event),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match self {
            SidePanel::Attribute(c) => c.render(frame, area),
            SidePanel::FileInfo(c) => c.render(frame, area),
            SidePanel::Bookmark(c) => c.render(frame, area),
            SidePanel::Grep(c) => c.render(frame, area),
            SidePanel::Settings(c) => c.render(frame, area),
            SidePanel::Tree(c) => c.render(frame, area),
            SidePanel::Preview(c) => c.render(frame, area),
            SidePanel::AudioPlayer(c) => c.render(frame, area),
            SidePanel::ImagePreview(c) => c.render(frame, area),
            SidePanel::Help(c) => c.render(frame, area),
        }
    }

    fn tick(&mut self) {
        match self {
            SidePanel::Attribute(c) => c.tick(),
            SidePanel::FileInfo(c) => c.tick(),
            SidePanel::Bookmark(c) => c.tick(),
            SidePanel::Grep(c) => c.tick(),
            SidePanel::Settings(c) => c.tick(),
            SidePanel::Tree(c) => c.tick(),
            SidePanel::Preview(c) => c.tick(),
            SidePanel::AudioPlayer(c) => c.tick(),
            SidePanel::ImagePreview(c) => c.tick(),
            SidePanel::Help(c) => c.tick(),
        }
    }

    fn keymap(&self) -> &'static str {
        match self {
            SidePanel::Attribute(c) => c.keymap(),
            SidePanel::FileInfo(c) => c.keymap(),
            SidePanel::Bookmark(c) => c.keymap(),
            SidePanel::Grep(c) => c.keymap(),
            SidePanel::Settings(c) => c.keymap(),
            SidePanel::Tree(c) => c.keymap(),
            SidePanel::Preview(c) => c.keymap(),
            SidePanel::AudioPlayer(c) => c.keymap(),
            SidePanel::ImagePreview(c) => c.keymap(),
            SidePanel::Help(c) => c.keymap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_delegates_to_the_active_panel() {
        let help = HelpComponent::new();
        let expected = help.keymap();
        let panel = SidePanel::Help(help);

        assert_eq!(panel.keymap(), expected);
        assert!(!expected.is_empty(), "panel keymap should not be empty");
    }
}
