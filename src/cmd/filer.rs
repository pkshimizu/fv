use crate::fs::VFile;
use crate::state::{
    AppState, BookmarkState, ConfirmAction, FileAction, PromptMode, SelectAction, SortKey,
    TextAction,
};
use crate::store::RootStore;
use anyhow::Result;

pub fn change_to_parent(state: &mut AppState) -> Result<()> {
    state.filer.change_dir_in_parent_dir()
}

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    state.filer.prev();
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    state.filer.next();
    Ok(())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    state.filer.first();
    Ok(())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    state.filer.last();
    Ok(())
}

pub fn enter_file(state: &mut AppState) -> Result<()> {
    let Some(file) = state.filer.selected_file() else {
        return Ok(());
    };
    let is_dir = file.is_dir();
    let path = file.absolute_path().to_string();
    if is_dir {
        state.filer.change_to(&path)?;
    } else {
        open::that(path)?;
    }
    Ok(())
}

pub fn prompt_copy(state: &mut AppState) -> Result<()> {
    start_file_input(state, "Copy to", |files| FileAction::Copy { files })
}

pub fn prompt_delete(state: &mut AppState) -> Result<()> {
    let files = collect_action_targets(state);
    if !files.is_empty() {
        let title = action_title("Delete", &files);
        state.prompt = PromptMode::Confirm {
            title,
            action: ConfirmAction::Delete { files },
        };
    }
    Ok(())
}

pub fn prompt_mkdir(state: &mut AppState) -> Result<()> {
    let dir = state.filer.current_dir.clone();
    if let Some(file_name) = dir.file_name() {
        let title = format!("Create directory in {file_name}");
        state.prompt = PromptMode::Text {
            title,
            action: TextAction::Mkdir { dir },
            value: String::new(),
        };
    }
    Ok(())
}

pub fn prompt_move(state: &mut AppState) -> Result<()> {
    start_file_input(state, "Move to", |files| FileAction::Move { files })
}

pub fn prompt_rename(state: &mut AppState) -> Result<()> {
    let selected_file = state.filer.selected_file();
    if let Some(selected_file) = selected_file {
        if let Some(file_name) = selected_file.file_name() {
            let title = format!("Rename {file_name}");
            state.prompt = PromptMode::Text {
                title,
                action: TextAction::Rename {
                    file: selected_file.clone(),
                },
                value: file_name.to_string(),
            };
        }
    }
    Ok(())
}

pub fn prompt_sort(state: &mut AppState) -> Result<()> {
    let options: Vec<String> = SortKey::ALL.iter().map(|k| k.label().to_string()).collect();
    let selected_index = state.filer.sort_key.index();
    state.prompt = PromptMode::Select {
        title: "Sort by".to_string(),
        options,
        selected_index,
        action: SelectAction::Sort,
    };
    Ok(())
}

pub fn prompt_search(state: &mut AppState) -> Result<()> {
    let original_index = state.filer.file_table_state.selected();
    state.prompt = PromptMode::Search {
        title: "Search".to_string(),
        value: String::new(),
        original_index,
    };
    Ok(())
}

pub fn prompt_grep(state: &mut AppState) -> Result<()> {
    state.prompt = PromptMode::Text {
        title: "Grep".to_string(),
        value: String::new(),
        action: TextAction::Grep,
    };
    Ok(())
}

pub fn add_bookmark(state: &AppState, store: &mut RootStore) -> Result<()> {
    if let Some(selected_file) = state.filer.selected_file() {
        store.bookmark.add(selected_file.absolute_path())?;
    }
    Ok(())
}

pub fn remove_bookmark(state: &AppState, store: &mut RootStore) -> Result<()> {
    if let Some(selected_file) = state.filer.selected_file() {
        store.bookmark.remove(selected_file.absolute_path())?;
    }
    Ok(())
}

pub fn show_bookmark(state: &mut AppState, store: &mut RootStore) -> Result<()> {
    state.bookmark = Some(BookmarkState::new(
        store.bookmark.get_paths().cloned().collect(),
    ));
    Ok(())
}

pub fn refresh_files(state: &mut AppState) -> Result<()> {
    state.filer.refresh_files()
}

pub fn toggle_checked_file(state: &mut AppState) -> Result<()> {
    state.filer.toggle_checked_file();
    state.filer.next();
    Ok(())
}

pub fn toggle_dot_files(state: &mut AppState) -> Result<()> {
    state.filer.toggle_show_dot_file()
}

fn start_file_input(
    state: &mut AppState,
    label: &str,
    make_action: impl FnOnce(Vec<VFile>) -> FileAction,
) -> Result<()> {
    let files = collect_action_targets(state);
    if !files.is_empty() {
        let title = action_title(label, &files);
        let init_value = if files.len() == 1 {
            files[0].absolute_path()
        } else {
            state.filer.current_dir.absolute_path()
        };
        state.prompt = PromptMode::File {
            title,
            value: init_value.to_string(),
            candidates: Vec::new(),
            candidate_index: None,
            action: make_action(files),
        };
    }
    Ok(())
}

fn collect_action_targets(state: &AppState) -> Vec<VFile> {
    if state.filer.checked_paths.is_empty() {
        state.filer.selected_file().cloned().into_iter().collect()
    } else {
        state
            .filer
            .current_dir_files
            .iter()
            .filter(|file| state.filer.checked_paths.contains(file.absolute_path()))
            .cloned()
            .collect()
    }
}

fn action_title(action_name: &str, files: &[VFile]) -> String {
    if files.len() == 1 {
        format!(
            "{} {}?",
            action_name,
            files[0].file_name().unwrap_or("(unknown)")
        )
    } else {
        format!("{} {} files?", action_name, files.len())
    }
}
