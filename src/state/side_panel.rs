use crate::component::{
    AttributeComponent, BookmarkComponent, Component, FileInfoComponent, GrepComponent,
};

pub enum SidePanel {
    Bookmark(BookmarkComponent),
    Grep(GrepComponent),
    FileInfo(FileInfoComponent),
    Attribute(AttributeComponent),
}

impl SidePanel {
    /// Component trait への参照を返す
    pub fn as_component(&mut self) -> Option<&mut dyn Component> {
        match self {
            SidePanel::Attribute(c) => Some(c),
            SidePanel::FileInfo(c) => Some(c),
            SidePanel::Bookmark(c) => Some(c),
            SidePanel::Grep(c) => Some(c),
        }
    }

    /// Grep コンポーネントの非同期結果を受信する
    pub fn receive_async_results(&mut self) {
        if let SidePanel::Grep(grep) = self {
            grep.receive_results();
        }
    }
}
