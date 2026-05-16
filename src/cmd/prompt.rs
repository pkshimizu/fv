use crate::component::GrepComponent;
use crate::fs::VFile;
use crate::state::{
    AppState, ConfirmAction, FileAction, PromptMode, SelectAction, SidePanel, SortKey, TextAction,
};
use crate::store::RootStore;
use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::Path;

/// プロンプトの確定アクションを実行する。
/// PromptComponent の input_ok が Action::ExecutePrompt(PromptMode) を返し、
/// App::handle_action がこの関数を呼び出す。
pub fn execute_prompt_action(
    state: &mut AppState,
    store: &mut RootStore,
    input: PromptMode,
) -> Result<()> {
    let skip_clear = matches!(
        input,
        PromptMode::Select {
            action: SelectAction::Sort,
            ..
        }
    );
    match input {
        PromptMode::Confirm { action, .. } => execute_confirm_action(action),
        PromptMode::Text { action, value, .. } => {
            execute_text_action(state, store, action, value.as_str())
        }
        PromptMode::File { action, value, .. } => {
            execute_file_action(state, action, value.as_str())
        }
        PromptMode::Select {
            action,
            selected_index,
            ..
        } => execute_select_action(state, action, selected_index),
        PromptMode::None | PromptMode::Error { .. } | PromptMode::Search { .. } => Ok(()),
    }?;
    if !skip_clear {
        state.filer.checked_paths.clear();
    }
    Ok(())
}

fn execute_confirm_action(action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => execute_delete(files),
    }
}

fn execute_text_action(
    state: &mut AppState,
    store: &mut RootStore,
    action: TextAction,
    value: &str,
) -> Result<()> {
    match action {
        TextAction::Mkdir { dir } => dir.create_dir(value),
        TextAction::Touch { dir } => dir.create_file(value),
        TextAction::Rename { file } => {
            file.rename(value)?;
            state.filer.set_pending_select_name(value.to_string());
            Ok(())
        }
        TextAction::Zip { dir, files } => dir.create_zip(value, &files),
        TextAction::Grep => execute_grep(state, store, value),
    }
}

fn execute_file_action(state: &mut AppState, action: FileAction, value: &str) -> Result<()> {
    match action {
        FileAction::Copy { files } => {
            for file in &files {
                file.copy_to(value)?;
            }
            Ok(())
        }
        FileAction::Move { files } => {
            for file in &files {
                file.move_to(value)?;
            }
            Ok(())
        }
        FileAction::Jump => {
            let path = Path::new(value);
            anyhow::ensure!(path.is_dir(), "{value} はディレクトリではありません");
            state.filer.change_to(value)
        }
    }
}

fn execute_select_action(
    state: &mut AppState,
    action: SelectAction,
    selected_index: usize,
) -> Result<()> {
    match action {
        SelectAction::Sort => {
            if let Some(&sort_key) = SortKey::ALL.get(selected_index) {
                state.filer.sort_key = sort_key;
                state.filer.refresh_files()?;
            }
            Ok(())
        }
    }
}

fn execute_delete(files: Vec<VFile>) -> Result<()> {
    for file in &files {
        file.delete()?;
    }
    Ok(())
}

fn execute_grep(state: &mut AppState, _store: &mut RootStore, value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }

    let dir_path = state.filer.current_dir.absolute_path().to_string();
    let pattern = value.to_string();

    let mut child = std::process::Command::new("grep")
        .args([
            "-rlF",
            "--binary-files=without-match",
            "--",
            &pattern,
            &dir_path,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to execute grep")?;

    let stdout = child.stdout.take().context("Failed to take stdout")?;

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        let mut canceled = false;
        for line in reader.lines() {
            let Ok(path) = line else { break };
            if tx.send(path).is_err() {
                canceled = true;
                break;
            }
        }
        if canceled {
            let _ = child.kill();
        }
        let _ = child.wait();
    });

    state.side_panel = Some(SidePanel::Grep(GrepComponent::new(rx)));
    Ok(())
}
