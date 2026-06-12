use crate::fs::VFile;
use crate::state::ProgressMessage;
use crate::state::table_cursor::TableCursor;
use anyhow::{Context, Result};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SortKey {
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
    DateAsc,
    DateDesc,
}

impl SortKey {
    pub const ALL: [SortKey; 6] = [
        SortKey::NameAsc,
        SortKey::NameDesc,
        SortKey::SizeAsc,
        SortKey::SizeDesc,
        SortKey::DateAsc,
        SortKey::DateDesc,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SortKey::NameAsc => "Name ↑",
            SortKey::NameDesc => "Name ↓",
            SortKey::SizeAsc => "Size ↑",
            SortKey::SizeDesc => "Size ↓",
            SortKey::DateAsc => "Date ↑",
            SortKey::DateDesc => "Date ↓",
        }
    }

    pub fn index(&self) -> usize {
        SortKey::ALL.iter().position(|k| k == self).unwrap_or(0)
    }

    fn is_apply_for_dirs(&self) -> bool {
        !matches!(self, SortKey::SizeAsc | SortKey::SizeDesc)
    }

    fn compare(&self, a: &VFile, b: &VFile) -> Ordering {
        match self {
            SortKey::NameAsc => a.file_name().cmp(&b.file_name()),
            SortKey::NameDesc => b.file_name().cmp(&a.file_name()),
            SortKey::SizeAsc | SortKey::SizeDesc => {
                let sa = a.metadata().map(|m| m.file_size()).unwrap_or(0);
                let sb = b.metadata().map(|m| m.file_size()).unwrap_or(0);
                let ord = sa.cmp(&sb);
                if matches!(self, SortKey::SizeDesc) {
                    ord.reverse()
                } else {
                    ord
                }
            }
            SortKey::DateAsc | SortKey::DateDesc => {
                let da = a.metadata().ok().and_then(|m| m.modified().ok());
                let db = b.metadata().ok().and_then(|m| m.modified().ok());
                let ord = da.cmp(&db);
                if matches!(self, SortKey::DateDesc) {
                    ord.reverse()
                } else {
                    ord
                }
            }
        }
    }
}

#[derive(Debug)]
struct FilerFilter {
    show_dot_file: bool,
    /// 名前による絞り込みフィルタの問い合わせ文字列。空文字列はフィルタ無効。
    /// 大文字小文字を無視した部分一致で評価する。
    name: String,
}

impl FilerFilter {
    fn new() -> Self {
        Self {
            show_dot_file: false,
            name: String::new(),
        }
    }

    fn apply(&self, files: Vec<VFile>) -> Vec<VFile> {
        files
            .into_iter()
            .filter(|file| self.should_include(file))
            .collect()
    }

    fn should_include(&self, file: &VFile) -> bool {
        self.show_dot_file || file.file_name().is_none_or(Self::is_visible_name)
    }

    /// ファイル名がフィルタ条件（ドットファイル非表示）で可視かどうかを判定する。
    /// should_include と start_async_load 内で共通利用される。
    fn is_visible_name(name: &str) -> bool {
        !name.starts_with('.')
    }

    /// 名前フィルタにマッチするか（大文字小文字無視の部分一致）。
    /// `lower_query` は小文字化済みの問い合わせ文字列。空なら常にマッチ。
    /// クエリの小文字化はループ外で 1 回だけ行う想定（find_matching_index と同じ流儀）。
    fn matches_name(file: &VFile, lower_query: &str) -> bool {
        if lower_query.is_empty() {
            return true;
        }
        file.file_name()
            .is_some_and(|name| name.to_lowercase().contains(lower_query))
    }
}

/// Operation Targets を解決した結果。集合だけでなく、Cursor File 由来か
/// Checked Paths 由来かという「由来」を保持する（CONTEXT.md 参照）。
/// いずれの variant も非空を不変条件とし、ターゲットが存在しない場合は
/// `operation_targets` が `None` を返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationTargets {
    /// Checked Paths が空のため、Cursor File 単体を対象とする。
    Cursor(VFile),
    /// 多重選択。Checked Paths に一致する current_dir_files（常に非空）。
    Checked(Vec<VFile>),
}

impl OperationTargets {
    /// 実際に操作する VFile 列へ落とし込む。Copy / Move / Delete / Zip が利用する。
    pub fn into_files(self) -> Vec<VFile> {
        match self {
            OperationTargets::Cursor(file) => vec![file],
            OperationTargets::Checked(files) => files,
        }
    }

    /// 各ターゲットの絶対パス文字列列へ落とし込む。Yank がクリップボードへ書き出す際に利用する。
    /// current_dir_files の順序（into_files と同じ）を保つ。
    pub fn into_absolute_paths(self) -> Vec<String> {
        self.into_files()
            .into_iter()
            .map(|file| file.absolute_path().to_string())
            .collect()
    }
}

pub struct FilerState {
    pub current_dir: VFile,
    /// 表示中（ドットファイル＋名前フィルタ適用後）のファイル一覧。描画・カーソル・
    /// 選択・Operation Targets はすべてこれを参照する。
    pub current_dir_files: Vec<VFile>,
    /// 名前フィルタ適用中のみ、ドットファイルフィルタ後の全件を退避しておく backing。
    /// `filter.name` が空（フィルタ無効）のときは空で、`current_dir_files` が全件を兼ねる。
    all_files: Vec<VFile>,
    pub file_table_state: TableState,
    checked_paths: HashSet<String>,
    pub sort_key: SortKey,
    filter: FilerFilter,
    pending_select_name: Option<String>,
    dir_load_rx: Option<mpsc::Receiver<VFile>>,
    progress_rx: Option<mpsc::Receiver<ProgressMessage>>,
    load_error: Option<String>,
    prev_dir: Option<VFile>,
    /// 同一ディレクトリ更新（in-place refresh）中に受信ファイルを蓄積する一時バッファ。
    /// `Some` の間は旧リストを表示したまま受信を貯め、完了時に一括差し替えする
    /// （ちらつき防止）。ディレクトリ移動時は `None`（従来の逐次マージ）。
    loading_buffer: Option<Vec<VFile>>,
}

impl std::fmt::Debug for FilerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilerState")
            .field("current_dir", &self.current_dir)
            .field("current_dir_files", &self.current_dir_files.len())
            .field("sort_key", &self.sort_key)
            .field("pending_select_name", &self.pending_select_name)
            .field("is_loading", &self.dir_load_rx.is_some())
            .field("load_error", &self.load_error)
            .finish()
    }
}

impl FilerState {
    pub fn new() -> Self {
        Self {
            current_dir: VFile::new(""),
            current_dir_files: Vec::new(),
            all_files: Vec::new(),
            file_table_state: TableState::default(),
            checked_paths: HashSet::new(),
            sort_key: SortKey::NameAsc,
            filter: FilerFilter::new(),
            pending_select_name: None,
            dir_load_rx: None,
            progress_rx: None,
            load_error: None,
            prev_dir: None,
            loading_buffer: None,
        }
    }

    pub fn init(&mut self, startup_dir: Option<std::path::PathBuf>) -> Result<()> {
        let init_dir = if let Some(dir) = startup_dir {
            dir
        } else {
            std::env::current_dir()
                .ok()
                .or_else(dirs::home_dir)
                .context("Failed to get initial directory")?
        };
        let current_dir_path = init_dir.to_str().context("Failed to get path string")?;
        self.load_current_dir_sync(Some(VFile::new(current_dir_path)))?;

        self.file_table_state.select(Some(0));
        Ok(())
    }

    fn cursor(&mut self) -> TableCursor<'_> {
        TableCursor::new(&mut self.file_table_state, self.current_dir_files.len())
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }

    pub fn first(&mut self) {
        self.cursor().first();
    }

    pub fn last(&mut self) {
        self.cursor().last();
    }

    pub fn change_to(&mut self, path: &str) {
        self.start_async_load(Some(VFile::new(path)));
    }

    /// ホームディレクトリへ移動する。取得できない場合（dirs::home_dir() が None、
    /// またはパスが非 UTF-8）は何もしない。
    pub fn change_to_home(&mut self) {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        if let Some(home) = home.to_str() {
            self.change_to(home);
        }
    }

    pub fn change_dir_in_parent_dir(&mut self) {
        if let Some(parent_dir) = self.current_dir.parent_dir() {
            // 遷移元（今いるディレクトリ）の名前を控えておき、親ロード後に
            // finalize_loaded_files がその名前でカーソルを復元する。
            if let Some(name) = self.current_dir.file_name() {
                self.pending_select_name = Some(name.to_string());
            }
            self.start_async_load(Some(parent_dir));
        }
    }

    pub fn set_pending_select_name(&mut self, name: String) {
        self.pending_select_name = Some(name);
    }

    pub fn refresh_files(&mut self) {
        let selected_name = self.pending_select_name.take().or_else(|| {
            self.selected_file()
                .and_then(|f| f.file_name().map(String::from))
        });
        self.pending_select_name = selected_name;
        self.start_async_load(None);
    }

    pub fn selected_file(&self) -> Option<&VFile> {
        let selected_index = self.file_table_state.selected();
        selected_index.and_then(|i| self.current_dir_files.get(i))
    }

    pub fn is_checked(&self, file: &VFile) -> bool {
        self.is_checked_path(file.absolute_path())
    }

    fn is_checked_path(&self, path: &str) -> bool {
        self.checked_paths.contains(path)
    }

    pub fn toggle_checked_file(&mut self) {
        if let Some(selected_file) = self.selected_file() {
            let path = selected_file.absolute_path().to_string();
            if self.is_checked_path(&path) {
                self.checked_paths.remove(&path);
            } else {
                self.checked_paths.insert(path);
            }
        }
    }

    /// 全選択とクリアを切り替える。チェック済みが 1 件以上あればクリア、
    /// 0 件なら表示中（current_dir_files）のすべてを Checked Paths に加える。
    pub fn toggle_check_all(&mut self) {
        if self.checked_paths.is_empty() {
            self.check_all_files();
        } else {
            self.clear_checked_paths();
        }
    }

    /// 表示中（current_dir_files）のすべてのファイルを Checked Paths に加える。
    fn check_all_files(&mut self) {
        self.checked_paths.extend(
            self.current_dir_files
                .iter()
                .map(|file| file.absolute_path().to_string()),
        );
    }

    /// Checked Paths をすべて解除する。
    pub fn clear_checked_paths(&mut self) {
        self.checked_paths.clear();
    }

    /// Operation Targets を解決する。「Checked Paths が非空ならそれ、さもなくば
    /// Cursor File 単体」というルール（CONTEXT.md 参照）の唯一の実装。
    /// ターゲットが存在しない場合は `None`。
    pub fn operation_targets(&self) -> Option<OperationTargets> {
        if self.checked_paths.is_empty() {
            self.selected_file().cloned().map(OperationTargets::Cursor)
        } else {
            let files: Vec<VFile> = self
                .current_dir_files
                .iter()
                .filter(|file| self.is_checked_path(file.absolute_path()))
                .cloned()
                .collect();
            (!files.is_empty()).then_some(OperationTargets::Checked(files))
        }
    }

    pub fn show_dot_file(&self) -> bool {
        self.filter.show_dot_file
    }

    pub fn toggle_show_dot_file(&mut self) {
        self.filter.show_dot_file = !self.filter.show_dot_file;
        self.refresh_files();
    }

    /// 現在の名前フィルタの問い合わせ文字列（空文字列はフィルタ無効）。
    pub fn name_filter(&self) -> &str {
        &self.filter.name
    }

    /// 名前フィルタが有効か。
    pub fn is_filtering(&self) -> bool {
        !self.filter.name.is_empty()
    }

    /// 名前フィルタを設定する（インクリメンタル）。空文字列で解除して全件に戻す。
    /// ディスクは再読込せず、退避した全件（`all_files`）からメモリ内で表示集合を再計算する。
    pub fn set_name_filter(&mut self, query: &str) {
        let was_filtering = self.is_filtering();
        if query.is_empty() {
            if was_filtering {
                // 解除: 退避していた全件を表示集合へ戻す。
                self.current_dir_files = std::mem::take(&mut self.all_files);
                self.filter.name.clear();
                self.clamp_cursor();
            }
            return;
        }
        if !was_filtering {
            // フィルタ開始: 現在の全件を backing へ退避する（直後に表示集合を作り直す）。
            self.all_files = std::mem::take(&mut self.current_dir_files);
        }
        self.filter.name = query.to_string();
        self.rebuild_filtered_view();
        self.clamp_cursor();
    }

    /// `all_files`（全件）から名前フィルタを適用して表示集合を再構築する。
    /// フィルタ有効時のみ呼ぶ（`all_files` が backing を保持している前提）。
    fn rebuild_filtered_view(&mut self) {
        let lower_query = self.filter.name.to_lowercase();
        self.current_dir_files = self
            .all_files
            .iter()
            .filter(|file| FilerFilter::matches_name(file, &lower_query))
            .cloned()
            .collect();
    }

    /// ロード結果（`current_dir_files` に入った全件）に対し、フィルタが有効なら
    /// 全件を `all_files` へ退避して表示集合を絞り込む。フィルタ無効なら何もしない。
    /// ディレクトリ移動以外（in-place refresh・同期ロード）の完了時に呼ぶ。
    fn reapply_active_filter(&mut self) {
        if !self.is_filtering() {
            return;
        }
        self.all_files = std::mem::take(&mut self.current_dir_files);
        self.rebuild_filtered_view();
    }

    /// 名前フィルタの状態を解除する（ディレクトリ移動時に内容が別物になるため）。
    fn reset_name_filter(&mut self) {
        self.filter.name.clear();
        self.all_files.clear();
    }

    /// 表示集合の件数にカーソルを収める。範囲外なら末尾へ、空なら未選択にする。
    fn clamp_cursor(&mut self) {
        let len = self.current_dir_files.len();
        let index = if len == 0 {
            None
        } else {
            Some(self.file_table_state.selected().unwrap_or(0).min(len - 1))
        };
        self.file_table_state.select(index);
    }

    pub fn select_matching_file(&mut self, query: &str) {
        if let Some(i) = self.find_matching_index(query, 0, true) {
            self.file_table_state.select(Some(i));
        }
    }

    pub fn select_next_matching_file(&mut self, query: &str) {
        let current = self.file_table_state.selected().unwrap_or(0);
        if let Some(i) = self.find_matching_index(query, current.wrapping_add(1), true) {
            self.file_table_state.select(Some(i));
        }
    }

    pub fn select_prev_matching_file(&mut self, query: &str) {
        let current = self.file_table_state.selected().unwrap_or(0);
        if let Some(i) = self.find_matching_index(query, current.wrapping_sub(1), false) {
            self.file_table_state.select(Some(i));
        }
    }

    fn find_matching_index(&self, query: &str, start: usize, forward: bool) -> Option<usize> {
        super::list_search::find_matching_index(
            self.current_dir_files.len(),
            start,
            forward,
            query,
            |i| self.current_dir_files[i].file_name(),
        )
    }

    /// 起動時の同期ロード（init 専用）
    fn load_current_dir_sync(&mut self, current_dir: Option<VFile>) -> Result<()> {
        let mut files = if let Some(current_dir) = &current_dir {
            current_dir.list()?
        } else {
            self.current_dir.list()?
        };
        files = self.filter.apply(files);
        Self::sort_files(&mut files, self.sort_key);

        if let Some(current_dir) = current_dir {
            self.current_dir = current_dir;
        }

        self.current_dir_files = files;
        self.reapply_active_filter();
        Ok(())
    }

    /// ディレクトリ走査を別スレッドで実行し、結果をmpscチャネルで受信する。
    fn start_async_load(&mut self, new_dir: Option<VFile>) {
        // 既存のロードをキャンセル
        self.dir_load_rx = None;
        self.progress_rx = None;
        self.load_error = None;

        // ディレクトリ移動時は内容が別物なのでクリアして逐次表示する。
        // 同一ディレクトリ更新（new_dir = None）はちらつき防止のため旧リストを保持し、
        // 完了時に一括差し替えする（後述の reconcile）。
        let is_navigation = new_dir.is_some();
        self.prev_dir = Some(self.current_dir.clone());
        if let Some(new_dir) = new_dir {
            self.current_dir = new_dir;
        }

        if is_navigation {
            // 移動先は内容が別物なので名前フィルタは解除する（フィルタはディレクトリ単位）。
            self.reset_name_filter();
            self.current_dir_files.clear();
            self.file_table_state.select(None);
            self.loading_buffer = None;
        } else {
            // in-place refresh: 受信は別バッファへ貯め、完了時に差し替える。
            self.loading_buffer = Some(Vec::new());
        }

        let (file_tx, file_rx) = mpsc::channel::<VFile>();
        let (progress_tx, progress_rx) = mpsc::channel::<ProgressMessage>();

        self.dir_load_rx = Some(file_rx);
        self.progress_rx = Some(progress_rx);

        let dir_path = self.current_dir.absolute_path().to_string();
        let show_dot_file = self.filter.show_dot_file;

        std::thread::spawn(move || {
            let entries = match std::fs::read_dir(&dir_path) {
                Ok(entries) => entries,
                Err(e) => {
                    let _ = progress_tx.send(ProgressMessage::Error(format!("{e}")));
                    return;
                }
            };

            for entry in entries {
                let Ok(entry) = entry else { continue };

                // ドットファイルフィルタ（VFile構築前に判定して高速化）
                if !show_dot_file
                    && let Some(name) = entry.file_name().to_str()
                    && !FilerFilter::is_visible_name(name)
                {
                    continue;
                }

                let Some(path_str) = entry.path().to_str().map(String::from) else {
                    continue;
                };
                let vfile = VFile::new(path_str);
                if file_tx.send(vfile).is_err() {
                    return; // キャンセルされた
                }
            }
            let _ = progress_tx.send(ProgressMessage::Complete);
        });
    }

    /// tick ごとにチャネルからファイルを受信する
    pub fn receive_files(&mut self) {
        // 進捗チャネルからエラーを監視（Update/Complete は意図的に無視）
        if let Some(progress_rx) = &self.progress_rx {
            loop {
                match progress_rx.try_recv() {
                    Ok(ProgressMessage::Error(e)) => {
                        self.load_error = Some(e);
                        self.progress_rx = None;
                        self.dir_load_rx = None;
                        self.loading_buffer = None;
                        // エラー時は元のディレクトリに戻して同期リロード
                        if let Some(prev_dir) = self.prev_dir.take() {
                            if let Err(restore_err) = self.load_current_dir_sync(Some(prev_dir)) {
                                if let Some(err) = &mut self.load_error {
                                    err.push_str(&format!(" (restore failed: {restore_err})"));
                                }
                            } else {
                                self.file_table_state.select(Some(0));
                            }
                        }
                        return;
                    }
                    Ok(ProgressMessage::Complete) => {
                        self.progress_rx = None;
                        break;
                    }
                    Ok(_) => {} // Update は無視
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.progress_rx = None;
                        break;
                    }
                }
            }
        }

        let Some(rx) = &self.dir_load_rx else {
            return;
        };

        const MAX_RECV_PER_FRAME: usize = 200;
        let mut batch: Vec<VFile> = Vec::new();
        let mut disconnected = false;

        while batch.len() < MAX_RECV_PER_FRAME {
            match rx.try_recv() {
                Ok(file) => batch.push(file),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        // in-place refresh（同一ディレクトリ更新）は旧リストを表示したままバッファへ蓄積し、
        // 完了時に reconcile_refresh で一括差し替えする。ディレクトリ移動は旧リスト
        // （クリア済み）へ逐次マージする。
        if self.loading_buffer.is_some() {
            if let Some(buffer) = self.loading_buffer.as_mut() {
                buffer.append(&mut batch);
            }
            if disconnected {
                self.close_load_channels();
                self.reconcile_refresh();
            }
        } else {
            self.merge_batch_into_current(batch);
            if disconnected {
                self.close_load_channels();
                self.finalize_loaded_files();
            }
        }
    }

    /// 非同期ロードのチャネルを閉じる（完了時の共通後始末）。
    fn close_load_channels(&mut self) {
        self.dir_load_rx = None;
        self.progress_rx = None;
    }

    /// 受信バッチをソートして現在のリストへマージし、選択中ファイルを名前で追従させる。
    /// ディレクトリ移動時の逐次表示で使う（旧リストはクリア済み）。
    fn merge_batch_into_current(&mut self, mut batch: Vec<VFile>) {
        if batch.is_empty() {
            return;
        }
        let sort_key = self.sort_key;
        Self::sort_files(&mut batch, sort_key);

        // 選択位置の補正: マージ前の選択ファイル名を記録
        let selected_name = self
            .file_table_state
            .selected()
            .and_then(|i| self.current_dir_files.get(i))
            .and_then(|f| f.file_name().map(String::from));

        // ソート済み同士のマージ
        let existing_files = std::mem::take(&mut self.current_dir_files);
        let mut merged = Vec::with_capacity(existing_files.len() + batch.len());
        let mut existing = existing_files.into_iter().peekable();
        let mut incoming = batch.into_iter().peekable();

        while existing.peek().is_some() || incoming.peek().is_some() {
            let take_existing = match (existing.peek(), incoming.peek()) {
                (Some(a), Some(b)) => Self::compare_files(a, b, sort_key) != Ordering::Greater,
                (Some(_), None) => true,
                _ => false,
            };
            if take_existing {
                merged.push(existing.next().unwrap());
            } else {
                merged.push(incoming.next().unwrap());
            }
        }
        self.current_dir_files = merged;

        // 選択位置の復元（逐次表示中はバッチごとに同じファイルへ追従）
        if let Some(name) = selected_name {
            if let Some(idx) = self
                .current_dir_files
                .iter()
                .position(|f| f.file_name() == Some(name.as_str()))
            {
                self.file_table_state.select(Some(idx));
            }
        } else if self.file_table_state.selected().is_none() && !self.current_dir_files.is_empty() {
            self.file_table_state.select(Some(0));
        }
    }

    /// 非同期ロードのエラーを取り出す
    pub fn take_error(&mut self) -> Option<String> {
        self.load_error.take()
    }

    /// 非同期ロード完了後の選択復元・チェック済みパスのクリーンアップ
    /// （ファイルは receive_files でソート済み挿入されるためソート不要）
    fn finalize_loaded_files(&mut self) {
        self.prev_dir = None;
        // pending_select_name は change_dir_in_parent_dir（親遷移時の遷移元名）や
        // jump_to / refresh_files がセットする。一致する名前が無ければ先頭へ。
        let name = self.pending_select_name.take();
        self.restore_cursor(name, Some(0));
        self.cleanup_checked_paths();
    }

    /// ロード完了後にカーソルを復元する。控えた選択名 `name` が現リストにあればそこへ、
    /// 無ければ `fallback`（クランプ）へ、それも無ければ先頭へ。空リストでは未選択。
    fn restore_cursor(&mut self, name: Option<String>, fallback: Option<usize>) {
        let len = self.current_dir_files.len();
        let index = if len == 0 {
            None
        } else {
            name.and_then(|n| {
                self.current_dir_files
                    .iter()
                    .position(|f| f.file_name() == Some(n.as_str()))
            })
            .or_else(|| fallback.map(|i| i.min(len - 1)))
            .or(Some(0))
        };
        self.file_table_state.select(index);
    }

    /// 現存しないパスを Checked Paths から取り除く。フィルタ有効時は全件（`all_files`）を
    /// 基準にして、絞り込みで一時的に隠れているだけのチェックを誤って消さないようにする。
    fn cleanup_checked_paths(&mut self) {
        let source = if self.is_filtering() {
            &self.all_files
        } else {
            &self.current_dir_files
        };
        let file_paths: HashSet<&str> = source.iter().map(|f| f.absolute_path()).collect();
        self.checked_paths
            .retain(|path| file_paths.contains(path.as_str()));
    }

    /// in-place refresh の完了処理。蓄積したバッファをソートして一括差し替えし、
    /// カーソルを復元する（同名があればそのファイル、無ければ旧 index をクランプ）。
    /// Checked Paths は現存パスのみ残す。旧リストは差し替えの瞬間まで保持される。
    fn reconcile_refresh(&mut self) {
        self.prev_dir = None;
        let Some(mut files) = self.loading_buffer.take() else {
            return;
        };
        Self::sort_files(&mut files, self.sort_key);

        // 復元の手がかり: refresh_files が控えた旧選択名と、未クリアの旧 index。
        // 名前一致が無ければ旧 index にクランプして同じ位置に留める。
        let name = self.pending_select_name.take();
        let old_index = self.file_table_state.selected();

        self.current_dir_files = files;
        // in-place refresh では全件が入る。名前フィルタ有効なら表示集合を絞り直す。
        self.reapply_active_filter();
        self.restore_cursor(name, old_index);

        self.cleanup_checked_paths();
    }

    pub fn is_loading(&self) -> bool {
        self.dir_load_rx.is_some()
    }

    fn compare_files(a: &VFile, b: &VFile, sort_key: SortKey) -> Ordering {
        // ディレクトリ優先は常に維持
        let ord = match (a.is_dir(), b.is_dir()) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (true, true) if !sort_key.is_apply_for_dirs() => a.file_name().cmp(&b.file_name()),
            _ => sort_key.compare(a, b),
        };
        // 同値の場合はファイル名で安定化
        ord.then_with(|| a.file_name().cmp(&b.file_name()))
    }

    fn sort_files(files: &mut [VFile], sort_key: SortKey) {
        files.sort_by(|a, b| Self::compare_files(a, b, sort_key));
    }

    pub fn jump_to(&mut self, file_path: &str) -> Result<()> {
        let path = std::path::Path::new(file_path);
        let parent = path
            .parent()
            .and_then(|p| p.to_str())
            .context("Invalid path")?;
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            self.pending_select_name = Some(name.to_string());
        }
        self.change_to(parent);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 指定したパス列を current_dir_files に持ち、cursor を index に合わせた
    /// FilerState を組み立てる。VFile::new は存在しないパスでもパニックしない。
    fn state_with(files: &[&str], cursor: Option<usize>) -> FilerState {
        let mut state = FilerState::new();
        state.current_dir_files = files.iter().map(|p| VFile::new(*p)).collect();
        state.file_table_state.select(cursor);
        state
    }

    #[test]
    fn operation_targets_falls_through_to_cursor_file_when_unchecked() {
        let state = state_with(&["/a/foo.txt", "/a/bar.txt"], Some(1));

        let targets = state.operation_targets();

        assert_eq!(
            targets,
            Some(OperationTargets::Cursor(VFile::new("/a/bar.txt")))
        );
    }

    #[test]
    fn operation_targets_is_none_when_unchecked_and_no_cursor() {
        let state = state_with(&[], None);

        assert_eq!(state.operation_targets(), None);
    }

    #[test]
    fn operation_targets_uses_checked_paths_in_directory_order() {
        let mut state = state_with(&["/a/a.txt", "/a/b.txt", "/a/c.txt"], Some(0));
        // c を先に、a を後にチェックしても、結果は current_dir_files の順序を保つ。
        state.checked_paths.insert("/a/c.txt".to_string());
        state.checked_paths.insert("/a/a.txt".to_string());

        let targets = state.operation_targets();

        assert_eq!(
            targets,
            Some(OperationTargets::Checked(vec![
                VFile::new("/a/a.txt"),
                VFile::new("/a/c.txt"),
            ]))
        );
    }

    #[test]
    fn into_absolute_paths_of_cursor_yields_the_single_cursor_path() {
        let targets = OperationTargets::Cursor(VFile::new("/a/bar.txt"));

        assert_eq!(
            targets.into_absolute_paths(),
            vec!["/a/bar.txt".to_string()]
        );
    }

    #[test]
    fn into_absolute_paths_of_checked_yields_all_paths_in_order() {
        let targets =
            OperationTargets::Checked(vec![VFile::new("/a/a.txt"), VFile::new("/a/c.txt")]);

        assert_eq!(
            targets.into_absolute_paths(),
            vec!["/a/a.txt".to_string(), "/a/c.txt".to_string()]
        );
    }

    #[test]
    fn operation_targets_is_none_when_checked_paths_are_all_stale() {
        let mut state = state_with(&["/a/a.txt"], Some(0));
        // current_dir_files に存在しないパスだけがチェックされている状態。
        state.checked_paths.insert("/a/gone.txt".to_string());

        assert_eq!(state.operation_targets(), None);
    }

    #[test]
    fn clear_checked_paths_empties_the_set() {
        let mut state = state_with(&["/a/a.txt"], Some(0));
        state.checked_paths.insert("/a/a.txt".to_string());

        state.clear_checked_paths();

        // チェックが消えたので Cursor File へフォールスルーする。
        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Cursor(VFile::new("/a/a.txt")))
        );
    }

    #[test]
    fn toggle_check_all_selects_every_displayed_file_when_none_checked() {
        let mut state = state_with(&["/a/a.txt", "/a/b.txt", "/a/c.txt"], Some(0));

        state.toggle_check_all();

        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Checked(vec![
                VFile::new("/a/a.txt"),
                VFile::new("/a/b.txt"),
                VFile::new("/a/c.txt"),
            ]))
        );
    }

    #[test]
    fn toggle_check_all_clears_when_any_file_is_checked() {
        let mut state = state_with(&["/a/a.txt", "/a/b.txt", "/a/c.txt"], Some(0));
        // 1 件でもチェック済みならクリアする。
        state.checked_paths.insert("/a/b.txt".to_string());

        state.toggle_check_all();

        // クリアされ、Cursor File へフォールスルーする。
        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Cursor(VFile::new("/a/a.txt")))
        );
    }

    /// current_dir_files の file_name 一覧を取り出すヘルパ。
    fn displayed_names(state: &FilerState) -> Vec<&str> {
        state
            .current_dir_files
            .iter()
            .filter_map(|f| f.file_name())
            .collect()
    }

    #[test]
    fn set_name_filter_narrows_to_case_insensitive_matches() {
        let mut state = state_with(&["/a/Foo.txt", "/a/bar.txt", "/a/foobar.md"], Some(0));

        state.set_name_filter("foo");

        assert!(state.is_filtering());
        assert_eq!(displayed_names(&state), vec!["Foo.txt", "foobar.md"]);
    }

    #[test]
    fn clearing_name_filter_restores_full_list() {
        let mut state = state_with(&["/a/foo.txt", "/a/bar.txt"], Some(0));

        state.set_name_filter("foo");
        assert_eq!(displayed_names(&state), vec!["foo.txt"]);

        state.set_name_filter("");
        assert!(!state.is_filtering());
        assert_eq!(displayed_names(&state), vec!["foo.txt", "bar.txt"]);
    }

    #[test]
    fn filter_limits_operation_targets_but_keeps_hidden_checks() {
        let mut state = state_with(&["/a/foo.txt", "/a/bar.txt"], Some(0));
        // 全件チェックしてから foo で絞り込む。
        state.toggle_check_all();
        state.set_name_filter("foo");

        // 操作対象は表示中（foo.txt）に限定される。bar.txt のチェックは保持される。
        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Checked(vec![VFile::new("/a/foo.txt")]))
        );

        // 解除すると隠れていた bar.txt のチェックも復活する。
        state.set_name_filter("");
        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Checked(vec![
                VFile::new("/a/foo.txt"),
                VFile::new("/a/bar.txt"),
            ]))
        );
    }

    #[test]
    fn toggle_check_all_selects_only_filtered_files() {
        let mut state = state_with(&["/a/foo.txt", "/a/bar.txt", "/a/foobar.md"], Some(0));
        state.set_name_filter("foo");

        // 表示中（foo.txt, foobar.md）だけが全選択される。
        state.toggle_check_all();

        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Checked(vec![
                VFile::new("/a/foo.txt"),
                VFile::new("/a/foobar.md"),
            ]))
        );
    }

    #[test]
    fn parent_navigation_selects_the_directory_we_came_from() {
        use std::time::Duration;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join("parent");
        for name in ["aaa", "bbb", "ccc"] {
            std::fs::create_dir_all(parent.join(name)).unwrap();
        }
        let child = parent.join("bbb");

        let mut state = FilerState::new();
        // bbb の中にいる状態を作る。
        state.current_dir = VFile::new(child.to_str().unwrap());

        // バックスペース相当: 親へ遷移する。
        state.change_dir_in_parent_dir();

        // 非同期ロードを完了まで駆動する（小さな dir なので実際は数 ms で終わる）。
        // 1 tick = 1ms なので MAX_TICKS は最大待機 ~5 秒の上限（ハング防止）。
        const MAX_TICKS: u32 = 5_000;
        let mut ticks = 0;
        while state.is_loading() && ticks < MAX_TICKS {
            state.receive_files();
            std::thread::sleep(Duration::from_millis(1));
            ticks += 1;
        }
        assert!(!state.is_loading(), "async load did not finish");

        // カーソルは遷移元 bbb に乗る（先頭 aaa ではなく）。
        assert_eq!(
            state.selected_file().and_then(|f| f.file_name()),
            Some("bbb")
        );
    }

    #[test]
    fn refresh_keeps_the_existing_list_visible() {
        // 同一ディレクトリ更新の開始直後、一覧はクリアされず旧内容・選択が残る（ちらつき防止）。
        let mut state = FilerState::new();
        state.current_dir = VFile::new("/some/dir");
        state.current_dir_files =
            vec![VFile::new("/some/dir/a.txt"), VFile::new("/some/dir/b.txt")];
        state.file_table_state.select(Some(1));

        state.refresh_files();

        assert!(
            !state.current_dir_files.is_empty(),
            "list must not be cleared on refresh"
        );
        assert_eq!(
            state.file_table_state.selected(),
            Some(1),
            "selection must be kept during refresh"
        );
    }

    /// 非同期ロードを完了まで駆動する（小さな dir なので実際は数 ms）。
    fn drive_until_loaded(state: &mut FilerState) {
        const MAX_TICKS: u32 = 5_000;
        let mut ticks = 0;
        while state.is_loading() && ticks < MAX_TICKS {
            state.receive_files();
            std::thread::sleep(std::time::Duration::from_millis(1));
            ticks += 1;
        }
        assert!(!state.is_loading(), "async load did not finish");
    }

    #[test]
    fn refresh_reflects_new_files_without_duplicates_and_keeps_cursor() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), b"a").unwrap();
        std::fs::write(tmp.path().join("b.txt"), b"b").unwrap();

        let mut state = FilerState::new();
        state.current_dir = VFile::new(tmp.path().to_str().unwrap());
        state.load_current_dir_sync(None).unwrap();
        state.file_table_state.select(Some(0)); // a.txt にカーソル
        assert_eq!(
            state.selected_file().and_then(|f| f.file_name()),
            Some("a.txt")
        );

        // 外部で新規ファイルを追加
        std::fs::write(tmp.path().join("c.txt"), b"c").unwrap();

        state.refresh_files();
        drive_until_loaded(&mut state);

        // 重複なく a,b,c が反映される
        let names: Vec<&str> = state
            .current_dir_files
            .iter()
            .filter_map(|f| f.file_name())
            .collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "c.txt"]);
        // カーソルは同じ a.txt に留まる
        assert_eq!(
            state.selected_file().and_then(|f| f.file_name()),
            Some("a.txt")
        );
    }

    #[test]
    fn refresh_clamps_cursor_to_same_index_when_selected_file_removed() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        for name in ["a.txt", "b.txt", "c.txt"] {
            std::fs::write(tmp.path().join(name), b"x").unwrap();
        }

        let mut state = FilerState::new();
        state.current_dir = VFile::new(tmp.path().to_str().unwrap());
        state.load_current_dir_sync(None).unwrap();
        state.file_table_state.select(Some(1)); // b.txt にカーソル
        assert_eq!(
            state.selected_file().and_then(|f| f.file_name()),
            Some("b.txt")
        );

        // カーソル位置の b.txt を削除
        std::fs::remove_file(tmp.path().join("b.txt")).unwrap();

        state.refresh_files();
        drive_until_loaded(&mut state);

        // 一覧は a, c に。カーソルは同じ index 1 に留まる（＝今は c.txt）。
        let names: Vec<&str> = state
            .current_dir_files
            .iter()
            .filter_map(|f| f.file_name())
            .collect();
        assert_eq!(names, vec!["a.txt", "c.txt"]);
        assert_eq!(state.file_table_state.selected(), Some(1));
        assert_eq!(
            state.selected_file().and_then(|f| f.file_name()),
            Some("c.txt")
        );
    }

    #[test]
    fn navigation_clears_the_list_during_load() {
        // ディレクトリ移動は内容が別物なので従来どおりクリア（逐次表示）する。
        let mut state = FilerState::new();
        state.current_dir = VFile::new("/old/dir");
        state.current_dir_files = vec![VFile::new("/old/dir/a.txt")];
        state.file_table_state.select(Some(0));

        state.change_to("/new/dir");

        assert!(
            state.current_dir_files.is_empty(),
            "navigation should clear the list"
        );
        assert_eq!(state.file_table_state.selected(), None);
    }

    #[test]
    fn change_to_home_sets_current_dir_to_home() {
        let home = dirs::home_dir().expect("home directory available in test env");

        let mut state = FilerState::new();
        state.current_dir = VFile::new("/some/other/dir");

        state.change_to_home();

        assert_eq!(
            state.current_dir.absolute_path(),
            home.to_str().unwrap(),
            "current_dir should become the home directory"
        );
    }
}
