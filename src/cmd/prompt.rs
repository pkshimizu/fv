use crate::fs::VFile;
use crate::state::{
    AppState, ConfirmAction, FileAction, FileActionCandidateType, PathListState, PromptMode,
    SelectAction, SidePanel, SortKey, TextAction,
};
use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::Path;

pub fn input_char(state: &mut AppState, c: char) -> Result<()> {
    match &mut state.prompt {
        PromptMode::Text { value, .. }
        | PromptMode::File { value, .. }
        | PromptMode::Search { value, .. } => {
            value.push(c);
        }
        _ => {}
    }
    after_input_value_changed(state);
    Ok(())
}

pub fn input_backspace(state: &mut AppState) -> Result<()> {
    match &mut state.prompt {
        PromptMode::Text { value, .. }
        | PromptMode::File { value, .. }
        | PromptMode::Search { value, .. } => {
            value.pop();
        }
        _ => {}
    }
    after_input_value_changed(state);
    Ok(())
}

fn after_input_value_changed(state: &mut AppState) {
    state.prompt.reset_candidates();
    if let PromptMode::Search { value, .. } = &state.prompt {
        state.filer.select_matching_file(value);
    }
}

pub fn input_select_left(state: &mut AppState) -> Result<()> {
    if let PromptMode::Select {
        selected_index,
        options,
        ..
    } = &mut state.prompt
    {
        if *selected_index > 0 {
            *selected_index -= 1;
        } else {
            *selected_index = options.len().saturating_sub(1);
        }
    }
    Ok(())
}

pub fn input_select_right(state: &mut AppState) -> Result<()> {
    if let PromptMode::Select {
        selected_index,
        options,
        ..
    } = &mut state.prompt
    {
        if *selected_index + 1 < options.len() {
            *selected_index += 1;
        } else {
            *selected_index = 0;
        }
    }
    Ok(())
}

pub fn input_tab(state: &mut AppState) -> Result<()> {
    if let PromptMode::File {
        value,
        candidate_type,
        candidates,
        candidate_index,
        ..
    } = &mut state.prompt
    {
        let compute = match candidate_type {
            FileActionCandidateType::All => compute_all_path_candidates,
            FileActionCandidateType::Directory => compute_dir_path_candidates,
        };
        cycle_candidates(
            value,
            candidates,
            candidate_index,
            CycleDirection::Forward,
            Some(compute),
        )?;
    }
    Ok(())
}

pub fn input_back_tab(state: &mut AppState) -> Result<()> {
    if let PromptMode::File {
        value,
        candidate_type,
        candidates,
        candidate_index,
        ..
    } = &mut state.prompt
    {
        let compute = match candidate_type {
            FileActionCandidateType::All => compute_all_path_candidates,
            FileActionCandidateType::Directory => compute_dir_path_candidates,
        };
        cycle_candidates(
            value,
            candidates,
            candidate_index,
            CycleDirection::Backward,
            Some(compute),
        )?;
    }
    Ok(())
}

type ComputeCandidates = fn(&str) -> Result<Vec<String>>;

#[derive(Debug)]
enum CycleDirection {
    Forward,
    Backward,
}

fn cycle_candidates(
    value: &mut String,
    candidates: &mut Vec<String>,
    candidate_index: &mut Option<usize>,
    direction: CycleDirection,
    compute: Option<ComputeCandidates>,
) -> Result<()> {
    if candidates.is_empty() {
        if let Some(compute) = compute {
            *candidates = compute(value)?;
            if !candidates.is_empty() {
                let start = match direction {
                    CycleDirection::Forward => 0,
                    CycleDirection::Backward => candidates.len() - 1,
                };
                *candidate_index = Some(start);
                *value = candidates[start].clone();
            }
        }
    } else if let Some(index) = candidate_index {
        let next = match direction {
            CycleDirection::Forward => (*index + 1) % candidates.len(),
            CycleDirection::Backward => {
                if *index == 0 {
                    candidates.len() - 1
                } else {
                    *index - 1
                }
            }
        };
        *candidate_index = Some(next);
        *value = candidates[next].clone();
    }
    Ok(())
}

pub fn input_ok(state: &mut AppState) -> Result<()> {
    // Search モードでは Enter でカーソル位置を維持したまま検索を終了する
    if matches!(state.prompt, PromptMode::Search { .. }) {
        state.prompt = PromptMode::None;
        return Ok(());
    }
    let input = std::mem::replace(&mut state.prompt, PromptMode::None);
    let skip_clear = matches!(
        input,
        PromptMode::Select {
            action: SelectAction::Sort,
            ..
        }
    );
    match input {
        PromptMode::Confirm { action, .. } => execute_confirm_action(state, action),
        PromptMode::Text { action, value, .. } => {
            execute_text_action(state, action, value.as_str())
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

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    if let PromptMode::Search { original_index, .. } = &state.prompt {
        state.filer.file_table_state.select(*original_index);
    }
    state.prompt = PromptMode::None;
    Ok(())
}

pub fn input_search_next(state: &mut AppState) -> Result<()> {
    if let PromptMode::Search { value, .. } = &state.prompt {
        state.filer.select_next_matching_file(value);
    }
    Ok(())
}

pub fn input_search_prev(state: &mut AppState) -> Result<()> {
    if let PromptMode::Search { value, .. } = &state.prompt {
        state.filer.select_prev_matching_file(value);
    }
    Ok(())
}

fn execute_confirm_action(_: &mut AppState, action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => execute_delete(files),
    }
}

fn execute_text_action(state: &mut AppState, action: TextAction, value: &str) -> Result<()> {
    match action {
        TextAction::Mkdir { dir } => execute_mkdir(dir, value),
        TextAction::Touch { dir } => execute_touch(dir, value),
        TextAction::Rename { file } => execute_rename(state, file, value),
        TextAction::Zip { dir, files } => execute_zip(dir, files, value),
        TextAction::Grep => execute_grep(state, value),
    }
}

fn execute_file_action(state: &mut AppState, action: FileAction, value: &str) -> Result<()> {
    match action {
        FileAction::Copy { files } => execute_copy(files, value),
        FileAction::Move { files } => execute_move(files, value),
        FileAction::Jump => execute_jump(state, value),
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

fn execute_copy(files: Vec<VFile>, value: &str) -> Result<()> {
    for file in &files {
        file.copy_to(value)?
    }
    Ok(())
}

fn execute_move(files: Vec<VFile>, value: &str) -> Result<()> {
    for file in &files {
        file.move_to(value)?
    }
    Ok(())
}

fn execute_jump(state: &mut AppState, value: &str) -> Result<()> {
    let path = Path::new(value);
    anyhow::ensure!(path.is_dir(), "{value} はディレクトリではありません");
    state.filer.change_to(value)?;
    Ok(())
}

fn execute_delete(files: Vec<VFile>) -> Result<()> {
    for file in &files {
        file.delete()?
    }
    Ok(())
}

fn execute_mkdir(dir: VFile, value: &str) -> Result<()> {
    dir.create_dir(value)?;
    Ok(())
}

fn execute_touch(dir: VFile, value: &str) -> Result<()> {
    dir.create_file(value)?;
    Ok(())
}

fn execute_zip(dir: VFile, files: Vec<VFile>, value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let zip_path = Path::new(dir.absolute_path()).join(value);
    anyhow::ensure!(
        !zip_path.exists(),
        "{}: File already exists",
        zip_path.display()
    );
    let zip_file = std::fs::File::create(&zip_path)
        .with_context(|| format!("{}: Failed to create zip file", zip_path.display()))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for file in &files {
        let file_path = Path::new(file.absolute_path());
        if file.is_dir() {
            add_dir_to_zip(&mut zip_writer, file_path, file_path, options)?;
        } else {
            let name = file.file_name().unwrap_or("unknown");
            zip_writer
                .start_file(name, options)
                .with_context(|| format!("Failed to add {name} to zip"))?;
            let mut f = std::fs::File::open(file_path)
                .with_context(|| format!("{}: Failed to open file", file_path.display()))?;
            std::io::copy(&mut f, &mut zip_writer)
                .with_context(|| format!("{}: Failed to write to zip", file_path.display()))?;
        }
    }
    zip_writer.finish()?;
    Ok(())
}

fn add_dir_to_zip(
    zip_writer: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("{}: Failed to read directory", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(base.parent().unwrap_or(base))
            .unwrap_or(&path);
        let name = relative.to_string_lossy();
        if path.is_dir() {
            zip_writer
                .add_directory(format!("{name}/"), options)
                .with_context(|| format!("Failed to add directory {name} to zip"))?;
            add_dir_to_zip(zip_writer, base, &path, options)?;
        } else {
            zip_writer
                .start_file(name.to_string(), options)
                .with_context(|| format!("Failed to add {name} to zip"))?;
            let mut f = std::fs::File::open(&path)
                .with_context(|| format!("{}: Failed to open file", path.display()))?;
            std::io::copy(&mut f, zip_writer)
                .with_context(|| format!("{}: Failed to write to zip", path.display()))?;
        }
    }
    Ok(())
}

fn execute_rename(state: &mut AppState, file: VFile, value: &str) -> Result<()> {
    file.rename(value)?;
    state.filer.set_pending_select_name(value.to_string());
    Ok(())
}

fn execute_grep(state: &mut AppState, value: &str) -> Result<()> {
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

    // grep実行時は既存のサイドパネルを置き換える（ユーザーが明示的に検索を実行した操作のため）
    state.side_panel = Some(SidePanel::Grep(PathListState::new(Vec::new(), Some(rx))));
    Ok(())
}

fn compute_all_path_candidates(input: &str) -> Result<Vec<String>> {
    compute_path_candidates(input, false)
}

fn compute_dir_path_candidates(input: &str) -> Result<Vec<String>> {
    compute_path_candidates(input, true)
}

fn compute_path_candidates(input: &str, dir_only: bool) -> Result<Vec<String>> {
    let path = Path::new(input);
    let (dir_path, prefix) = if input.ends_with('/') {
        (path.to_path_buf(), String::new())
    } else {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .context("Failed to get parent directory")?;
        let prefix = path
            .file_name()
            .context("Failed to get file name")?
            .to_string_lossy()
            .to_string();
        (dir, prefix)
    };

    let files = VFile::new(dir_path.to_string_lossy()).list()?;

    let mut candidates: Vec<String> = files
        .into_iter()
        .filter_map(|f| {
            if dir_only && !f.is_dir() {
                return None;
            }
            let name = f.file_name()?;
            if !name.starts_with(&prefix) {
                return None;
            }
            let mut s = f.absolute_path().to_string();
            if f.is_dir() {
                s.push('/');
            }
            Some(s)
        })
        .collect();

    candidates.sort();
    Ok(candidates)
}
