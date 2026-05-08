use crate::state::AttributeState;
use crate::state::PathListState;

#[derive(Debug)]
pub enum SidePanel {
    Bookmark(PathListState),
    Grep(PathListState),
    Attribute(AttributeState),
}
