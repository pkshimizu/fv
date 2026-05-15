use crate::component::{AttributeComponent, Component, FileInfoComponent};
use crate::state::PathListState;

// Component を含むため Debug は手動実装しない
pub enum SidePanel {
    Bookmark(PathListState),
    Grep(PathListState),
    FileInfo(FileInfoComponent),
    Attribute(AttributeComponent),
}

impl SidePanel {
    /// Component trait で処理するサイドパネルかどうかを返す
    pub fn is_component(&self) -> bool {
        matches!(self, SidePanel::Attribute(_) | SidePanel::FileInfo(_))
    }

    /// コンポーネントベースのサイドパネルの場合、Component trait への参照を返す
    pub fn as_component(&mut self) -> Option<&mut dyn Component> {
        match self {
            SidePanel::Attribute(c) => Some(c),
            SidePanel::FileInfo(c) => Some(c),
            _ => None,
        }
    }
}
