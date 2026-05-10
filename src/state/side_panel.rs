use crate::state::AttributeState;
use crate::state::PathListState;
use crate::state::TextOutputState;

#[derive(Debug)]
pub enum SidePanel {
    Bookmark(PathListState),
    Grep(PathListState),
    Shell(TextOutputState),
    Attribute(AttributeState),
}
