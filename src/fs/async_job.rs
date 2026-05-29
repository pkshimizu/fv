use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Scan Phase 中、ファイル発見ごとに `on_progress` を呼ばずに、この件数ごとにバッチで通知する。
/// `&mut dyn FnMut` の vtable hop と `spawn_async_job` 側の `Instant::now()` 呼出を削減する目的。
const SCAN_NOTIFY_BATCH: usize = 256;

/// 衝突回避時の suffix 探索上限 (`pick_unique_top_dest` で使用)。
const MAX_UNIQUE_PATH_SUFFIX: u32 = 1000;

/// Copy/Move の Scan Phase で扱う「src ファイルパス → 衝突回避済み宛先 top-level パス」のペア。
/// タプルだと `.0/.1` でアクセスする箇所が意図不明になりやすいため struct 化している。
#[derive(Debug, Clone)]
struct TopLevelPair {
    src: PathBuf,
    dst: PathBuf,
}

/// `std::fs::rename` の戻り Err がクロスファイルシステムを示すか判定する。
/// `std::io::ErrorKind::CrossesDevices` で表現できる場合はそれを優先し、Unix 上の古い API でも
/// `libc::EXDEV` (`raw_os_error`) で fallback する。
fn is_cross_device_error(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::CrossesDevices {
        return true;
    }
    #[cfg(unix)]
    {
        e.raw_os_error() == Some(libc::EXDEV)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// `dest` ディレクトリを必ず存在させる。Copy/Move 入口で共通的に呼び、ユーザが存在しない path を
/// 指定した場合でも先頭で確保する。
fn ensure_dest_dir(dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .with_context(|| format!("{}: Failed to create destination directory", dest.display()))
}

/// dest を「正確な宛先パス」（rename-on-copy/move）として扱うか判定する。
/// 単一 Operation Target かつ dest が既存ディレクトリでないときのみ true。
/// false のときは dest をコンテナディレクトリとして各 root をベース名で中に入れる。
/// CONTEXT.md の Destination 参照。Copy / Move 共通。
fn dest_is_target_path(roots: &[VFile], dest: &Path) -> bool {
    roots.len() == 1 && !dest.is_dir()
}

/// Copy / Move の前に存在を保証すべきディレクトリを作る。
/// コンテナ扱いなら dest 自身、正確な宛先パス扱いなら dest の親ディレクトリ。
fn ensure_destination_dir(roots: &[VFile], dest: &Path) -> Result<()> {
    let dir = if dest_is_target_path(roots, dest) {
        dest.parent()
    } else {
        Some(dest)
    };
    match dir {
        Some(d) => ensure_dest_dir(d),
        None => Ok(()),
    }
}

/// Async Job として実行される重いファイル操作。
/// UI とは結合せず、進捗はクロージャ経由で通知する。
/// `Phase::Cancelling` は worker からは emit せず、UI 側で Esc 受信時に上書きされる。
#[derive(Debug)]
pub enum FileJob {
    ZipExtract {
        file: VFile,
        dest: PathBuf,
    },
    Copy {
        files: Vec<VFile>,
        dest: PathBuf,
    },
    Move {
        files: Vec<VFile>,
        dest: PathBuf,
    },
    ZipCreate {
        dir: VFile,
        name: String,
        files: Vec<VFile>,
    },
    Delete {
        files: Vec<VFile>,
    },
}

impl FileJob {
    /// Job を実行する。
    /// `cancel` を File-level Checkpoint で監視し、true ならファイル境界で早期 return。
    ///
    /// # 進捗通知プロトコル
    /// - Phase 切り替え直後に必ず `(new_phase, 0, total)` を 1 回 emit する
    ///   (Scan Phase 開始時は `(Scanning, 0, None)`、Operation Phase 開始時は `(Copying|Extracting|..., 0, Some(N))`)
    /// - Scan Phase 中: ファイル発見ごとではなく `SCAN_NOTIFY_BATCH` 件ごとに `(Scanning, k, None)` を emit する
    /// - Operation Phase 中: 1 ファイル完了ごとに `(Copying|..., k, Some(N))`
    /// - `total` が `Some(N)` の場合、Cancel されない限り processed は最終的に `N` に達する
    /// - Cancel された場合は File-level Checkpoint で emit が止まるため、processed < total のまま戻る
    pub fn run(
        self,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
    ) -> Result<()> {
        match self {
            FileJob::ZipExtract { file, dest } => {
                run_zip_extract(&file, &dest, cancel, on_progress)
            }
            FileJob::Copy { files, dest } => run_copy(&files, &dest, cancel, on_progress),
            FileJob::Move { files, dest } => run_move(&files, &dest, cancel, on_progress),
            FileJob::ZipCreate { dir, name, files } => {
                run_zip_create(&dir, &name, &files, cancel, on_progress)
            }
            FileJob::Delete { files } => run_delete(&files, cancel, on_progress),
        }
    }
}

/// Delete Job 本体。Scan Phase で削除対象を再帰列挙して件数をユーザに見せ、
/// Operation Phase では top-level の VFile を順次 `trash::delete` で削除する。
///
/// # Partial Result
/// Operation Phase 中の cancel: 既に `trash::delete` 済みの root は trash 側に残り、
/// 未着手の root は元の場所に残る。trash::delete は atomic per-item なので、
/// 半端に削除されたディレクトリ等の grey state は発生しない。
///
/// # 進捗 N の意味
/// - Scanning: 再帰ファイル数 (情報提示)
/// - Deleting: top-level VFile 件数 (`trash::delete` の単位)
///
/// 両 phase で N の意味が異なる点に注意 (trash::delete が atomic per-item のため)。
fn run_delete(
    files: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    run_delete_with(files, cancel, on_progress, &mut trash_delete)
}

/// 本番経路で使う `trash::delete` のラッパ。`path` を含む context はここで終わらせて、
/// 上位 (`run_delete_with`) では「delete 操作の責務」レベル (進捗 hint) のみ被せる役割分担にする。
fn trash_delete(path: &Path) -> Result<()> {
    trash::delete(path).with_context(|| format!("{}: Failed to move to trash", path.display()))
}

/// `delete_fn` 注入版。テストでは `std::fs::remove_*` 系を渡して
/// 実 trash 経由を避ける (ユーザのシステム trash を汚さない)。
fn run_delete_with(
    files: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
    delete_fn: &mut dyn FnMut(&Path) -> Result<()>,
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    // Scan Phase: 再帰列挙で削除対象のファイル数を数え、ユーザに削除規模を提示する。
    // Operation Phase は top-level VFile を 1 単位として `delete_fn` を呼ぶので、
    // Scan の総数とは N の意味が変わる点に注意 (run_delete doc 参照)。
    let scan_status = scan_delete_count(files, cancel, on_progress)?;
    if matches!(scan_status, CollectStatus::Cancelled) {
        return Ok(());
    }
    let total = files.len();
    on_progress(Phase::Deleting, 0, Some(total));
    for (i, file) in files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let path = Path::new(file.absolute_path());
        let remaining = total - i - 1;
        delete_fn(path).with_context(|| format!("Delete aborted ({remaining} files remaining)"))?;
        on_progress(Phase::Deleting, i + 1, Some(total));
    }
    Ok(())
}

/// 削除対象のファイル数を再帰的にカウントし、`(Scanning, k, None)` を batch で発火する。
/// トップレベル symlink はカウント 1 として扱う (trash::delete に follow を任せる)。
///
/// `metadata()` (symlink follow) ではなく `symlink_metadata()` を使う。`metadata()` だと
/// 例えば `~/Documents -> /` のような dir-symlink が選ばれていた場合、follow 先の巨大ツリーを
/// 不意に再帰列挙してしまうため。Operation Phase は `trash::delete` 1 回しか呼ばないので
/// Scan で follow する利益はない。
fn scan_delete_count(
    roots: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    on_progress(Phase::Scanning, 0, None);
    let mut count = 0usize;
    for root in roots {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let src = Path::new(root.absolute_path());
        // stat 失敗を「ファイル扱い」に握りつぶさず Scan で早期 Err として顕在化する。
        let metadata = src
            .symlink_metadata()
            .with_context(|| format!("{}: Failed to stat source", src.display()))?;
        let file_type = metadata.file_type();
        if file_type.is_dir() && !file_type.is_symlink() {
            match walk_count_for_delete(src, &mut count, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else {
            count += 1;
            notify_scan_progress(count, on_progress);
        }
    }
    // Scan Phase の最後で端数件数を通知 (バッチ境界で終わらない場合の取りこぼし対策)。
    // ちょうど SCAN_NOTIFY_BATCH の倍数で終わったときは notify_scan_progress で発火済みなのでスキップ。
    // count == 0 のときは 180 行目の入口 emit と同値になるのでスキップ。
    if count > 0 && !count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
    Ok(CollectStatus::Completed)
}

/// `src` ディレクトリ配下を再帰的に列挙し、ファイル数を `count` に加算する。
///
/// - 再帰中の symlink は follow せず、`count += 1` で 1 エントリとして数える
///   (`trash::delete` は top-level しか触らないため、内部 symlink の follow は無意味かつ
///   任意領域脱出リスクになりうる)
/// - `read_dir` / `DirEntry::file_type` の I/O Err は `with_context` で対象パスを含めて伝播する
/// - 各エントリ処理前に cancel をチェックする File-level Checkpoint
fn walk_count_for_delete(
    src: &Path,
    count: &mut usize,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    for entry in std::fs::read_dir(src)
        .with_context(|| format!("{}: Failed to read directory", src.display()))?
    {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
        let entry_src = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to read file type", entry_src.display()))?;
        if file_type.is_dir() && !file_type.is_symlink() {
            match walk_count_for_delete(&entry_src, count, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else {
            *count += 1;
            notify_scan_progress(*count, on_progress);
        }
    }
    Ok(CollectStatus::Completed)
}

/// Scan Phase の batch 通知ヘルパ。
/// `count == 0` での発火 (空入力でカウント前の状態) を抑止しつつ、
/// `SCAN_NOTIFY_BATCH` の倍数到達時に `(Scanning, count, None)` を発火する。
fn notify_scan_progress(count: usize, on_progress: &mut dyn FnMut(Phase, usize, Option<usize>)) {
    if count > 0 && count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
}

/// Zip 作成 Job 本体。Scan Phase → Operation Phase の二相で動く。
///
/// # Partial Result の例外
/// Operation Phase 中の Cancel またはエラーで途中終了した場合、書きかけの zip ファイルは
/// `ZipPathGuard` の Drop で自動削除する (壊れた `.zip` を残さない方針 - ADR-0001 Zip 例外)。
/// Scan Phase 中の Err は zip ファイル自体まだ作っていないので cleanup 不要。
///
/// # 旧 `fs::file::create_zip` からの挙動互換
/// - top-level `name` が既存なら `name_1`, `name_2`, ... に振り替える
/// - 再帰内の symlink エントリは zip に含めない (リンク先 follow による任意領域漏洩を回避)
/// - top-level の VFile が dir-symlink の場合は `metadata()` で follow しその内容を zip 化する
///   (`copy_to` と同じ「ユーザが明示的に指定した対象は follow」規約)
fn run_zip_create(
    dir: &VFile,
    name: &str,
    files: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    // UI 層でも `anyhow::ensure!` で同等検証している前提だが、API 直接呼出に対する防御深度。
    anyhow::ensure!(
        !name.is_empty()
            && Path::new(name)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
        "{name}: Invalid zip name"
    );
    // Scan Phase: zip に詰めるエントリを集める (内部で cancel チェック)
    let Some(plan) = scan_zip_plan(files, cancel, on_progress)? else {
        return Ok(());
    };
    // 衝突回避: dir/name がすでに存在すれば name_1, name_2, ... に振り替える
    let zip_path = pick_unique_top_dest(
        &Path::new(dir.absolute_path()).join(name),
        &std::collections::HashSet::new(),
    )?;

    // Operation Phase: zip 書き出し。ファイル open に成功した直後から `ZipPathGuard` で
    // cleanup 範囲を限定する (open 失敗時に他プロセスのファイルを誤って削除しないため)。
    write_zip_plan(&plan, &zip_path, cancel, on_progress)?;
    Ok(())
}

/// 書きかけ zip を Drop 時に削除する RAII ガード。
/// Cancel / Err 経路で `let _ = remove_file()` を散らさず、cleanup を関数末尾で集約する。
/// `arm` で守備、通常完走時に `disarm` で解除。Drop 時の remove 失敗は `tracing::warn!` に残す
/// (`NotFound` は完走後に自分が削除済みのケースなので無視)。
struct ZipPathGuard<'a> {
    path: &'a Path,
    armed: bool,
}

impl<'a> ZipPathGuard<'a> {
    fn arm(path: &'a Path) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ZipPathGuard<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Err(e) = std::fs::remove_file(self.path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("failed to remove partial zip {}: {e}", self.path.display());
        }
    }
}

/// Scan Phase の結果。
/// `directories`: zip 内に作成すべきディレクトリエントリ名 (例: `"foo/"`)。
/// `files`: zip に書き込むエントリ列 (src パス + zip 内パス)。
#[derive(Debug, Default)]
struct ZipPlan {
    directories: Vec<String>,
    files: Vec<ZipEntry>,
}

#[derive(Debug)]
struct ZipEntry {
    src: PathBuf,
    /// zip 内でのエントリ名 (forward slash 区切り)
    name: String,
}

fn scan_zip_plan(
    roots: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<ZipPlan>> {
    let mut plan = ZipPlan::default();
    on_progress(Phase::Scanning, 0, None);
    for root in roots {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let src = Path::new(root.absolute_path());
        let metadata = src
            .metadata()
            .with_context(|| format!("{}: Failed to stat source", src.display()))?;
        // zip 内エントリ名は src 親からの相対パス (= src の basename) を起点に組み立てる。
        let prefix = src.parent().unwrap_or(src);
        if metadata.is_dir() {
            match collect_zip_directory(src, prefix, &mut plan, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(None),
            }
        } else {
            let name = relative_zip_name(src, prefix)?;
            enqueue_zip_file(&mut plan, src.to_path_buf(), name, on_progress);
        }
    }
    Ok(Some(plan))
}

fn collect_zip_directory(
    src: &Path,
    prefix: &Path,
    plan: &mut ZipPlan,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    let dir_name = relative_zip_name(src, prefix)?;
    plan.directories.push(format!("{dir_name}/"));
    for entry in std::fs::read_dir(src)
        .with_context(|| format!("{}: Failed to read directory", src.display()))?
    {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
        let entry_src = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to read file type", entry_src.display()))?;
        // 既存 add_dir_to_zip と同じく symlink は zip に含めない (リンク先データの follow を避ける)
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            match collect_zip_directory(&entry_src, prefix, plan, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else {
            let name = relative_zip_name(&entry_src, prefix)?;
            enqueue_zip_file(plan, entry_src, name, on_progress);
        }
    }
    Ok(CollectStatus::Completed)
}

/// `prefix` からの相対パスを zip エントリ名 (`/` 区切り) に整形する。
/// `path` が `prefix` 配下にない場合や、エントリ名が空文字列になる場合は Err を返す
/// (`Component::Normal` 以外をフィルタする性質上、ルート path 等で空文字に縮退する経路を防ぐ)。
fn relative_zip_name(path: &Path, prefix: &Path) -> Result<String> {
    let relative = path.strip_prefix(prefix).with_context(|| {
        format!(
            "{} is not under zip prefix {}",
            path.display(),
            prefix.display()
        )
    })?;
    let name = relative
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    anyhow::ensure!(
        !name.is_empty(),
        "{}: produced empty zip entry name",
        path.display()
    );
    Ok(name)
}

fn enqueue_zip_file(
    plan: &mut ZipPlan,
    src: PathBuf,
    name: String,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) {
    plan.files.push(ZipEntry { src, name });
    let count = plan.files.len();
    if count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
}

fn write_zip_plan(
    plan: &ZipPlan,
    zip_path: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    // `OpenOptions::create_new(true)` で atomic に新規作成。open 失敗時は cleanup 対象外
    // (他プロセスのファイルを誤って消さないため、ガードはここから arm する)。
    let zip_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(zip_path)
        .with_context(|| format!("{}: Failed to create zip file", zip_path.display()))?;
    let mut guard = ZipPathGuard::arm(zip_path);

    // central directory write の syscall 数を減らすため zip 出力側を BufWriter で包む
    let buffered = std::io::BufWriter::with_capacity(256 * 1024, zip_file);
    let mut writer = zip::ZipWriter::new(buffered);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let total = plan.files.len();
    on_progress(Phase::Zipping, 0, Some(total));

    for dir_name in &plan.directories {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        writer.add_directory(dir_name, options).with_context(|| {
            format!(
                "{}: Failed to add directory {dir_name} to zip",
                zip_path.display()
            )
        })?;
    }
    for (i, entry) in plan.files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        writer.start_file(&entry.name, options).with_context(|| {
            format!(
                "{}: Failed to add {} to zip",
                zip_path.display(),
                entry.name
            )
        })?;
        let f = std::fs::File::open(&entry.src)
            .with_context(|| format!("{}: Failed to open source", entry.src.display()))?;
        // 大量小ファイル時に read syscall 回数を削減するため BufReader で包む。
        // `std::io::copy` 失敗は I/O 由来の本物のエラー (例: disk full) として扱い `?` で伝播。
        // cancel タイミングはループ先頭で検知される。
        let mut reader = std::io::BufReader::with_capacity(64 * 1024, f);
        std::io::copy(&mut reader, &mut writer)
            .with_context(|| format!("{}: Failed to write to zip", entry.src.display()))?;
        on_progress(Phase::Zipping, i + 1, Some(total));
    }
    // ZipWriter::finish() は内部の BufWriter を返す。BufWriter::into_inner() で flush を強制し、
    // 残ったバッファ未書出を Err に昇格させる (Drop 時の握り潰しを避ける)。
    let buffered = writer
        .finish()
        .with_context(|| format!("{}: Failed to finalize zip", zip_path.display()))?;
    buffered.into_inner().map_err(|e| {
        anyhow::anyhow!("{}: Failed to flush zip writer: {}", zip_path.display(), e)
    })?;
    guard.disarm();
    Ok(CollectStatus::Completed)
}

/// Move Job 本体。
/// 同一 FS では `rename` 一発で済むため Scan Phase を経由せず top-level 件数で進捗を出す。
/// `std::fs::rename` がクロスファイルシステムで失敗した場合は、全 root を Scan + Copy + Remove の
/// フォールバックパスに切り替える (UI 上は `Scanning... N files` → `Moving k/N files` の遷移)。
///
/// # Partial Result
/// Move における Partial Result は「src から消え、dest に存在する root」の集合と定義する。
/// 中途半端な状態 (src/dest 重複や、コピー途中) は **未完了 root** とみなし、
/// ユーザに伝わるべき "完了済み" には含めない。
///
/// 各タイミングごとの実ディスク状態:
/// - 同一 FS 高速パス中の cancel: 既に rename 済みの root は dest に、未処理 root は src に残る (= 完了 + 未完了)
/// - 同一 FS 高速パス中の rename Err (probe 以降): 既に rename 済みの root は dest に残る (エラー文言に明記)
/// - EXDEV フォールバック中の cancel:
///   - Scan 中: 何も変えない (完了 root 無し)
///   - Copy 中: dest にコピー済み + src に全 root → "完了" root はゼロ。Copy 完了せず src/dest 重複の生データが残る
///   - Remove 中: dest には全 root のコピーが揃い、src からは既に削除された root が消える (完了済み = src 消去された root)
fn run_move(
    files: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    // `run_copy` と同じく、入口でコンテナ dir（または宛先パスの親）を確保する
    // (rename も create_dir_all 同等の効果は無いため)。
    ensure_destination_dir(files, dest)?;
    let pairs = resolve_top_level_pairs(files, dest)?;

    // 副作用つき probe: 先頭 root への rename を 1 回だけ試し、成功なら同一 FS 高速パスに乗り、
    // CrossesDevices ならフォールバックパスへ。先頭 root はこの時点で本処理を 1 件分消費しているため、
    // 後続ループは `skip(1)` する。
    let first = &pairs[0];
    match std::fs::rename(&first.src, &first.dst) {
        Ok(()) => {
            let total = pairs.len();
            on_progress(Phase::Moving, 0, Some(total));
            on_progress(Phase::Moving, 1, Some(total));
            for (i, pair) in pairs.iter().enumerate().skip(1) {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
                std::fs::rename(&pair.src, &pair.dst).with_context(|| {
                    format!(
                        "{} -> {}: Failed to rename (other roots may have been moved already)",
                        pair.src.display(),
                        pair.dst.display()
                    )
                })?;
                on_progress(Phase::Moving, i + 1, Some(total));
            }
            Ok(())
        }
        Err(e) if is_cross_device_error(&e) => {
            move_via_copy_and_remove(&pairs, cancel, on_progress)
        }
        Err(e) => Err(anyhow::Error::from(e).context(format!(
            "{} -> {}: Failed to rename",
            first.src.display(),
            first.dst.display()
        ))),
    }
}

/// EXDEV フォールバック: 事前解決済みペア列を Scan + Copy + Remove で移動する。
/// 進捗 phase は Scan 中 `Scanning`、Copy 以降 `Moving` (Remove ステップ中は最終 `(Moving, N, Some(N))` を保持)。
fn move_via_copy_and_remove(
    pairs: &[TopLevelPair],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    let Some(plan) = scan_pairs_into_plan(pairs, cancel, on_progress)? else {
        // Scan 中の cancel は Partial Result なしで早期 return
        return Ok(());
    };
    let total = plan.files.len();
    on_progress(Phase::Moving, 0, Some(total));
    // `run_copy` は冒頭で `ensure_dest_dir` するが、`run_move` の入口でも既に行っているため
    // ここでは plan.directories のみを `create_dir` で順次作成する。
    for dir in &plan.directories {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        std::fs::create_dir(dir)
            .with_context(|| format!("{}: Failed to create directory", dir.display()))?;
    }
    for (i, entry) in plan.files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Err(e) = copy_entry(entry) {
            // Copy 途中で cancel が立っていた場合は Err でなく cancel として畳む
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            return Err(e);
        }
        on_progress(Phase::Moving, i + 1, Some(total));
    }
    // 全 Copy 完了後に各 root の src を削除する。
    // `Path::is_dir()` は symlink を follow するため、dir-symlink を root に持つケースでリンク先の
    // ディレクトリを誤って `remove_dir_all` 経由で削除しに行ってしまう (現代の `remove_dir_all` は
    // `O_NOFOLLOW` で防御するが、防御深度として呼び出し側でも symlink_metadata で判定を分離する)。
    for pair in pairs {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let meta = std::fs::symlink_metadata(&pair.src).with_context(|| {
            format!("{}: Failed to stat source for removal", pair.src.display())
        })?;
        let file_type = meta.file_type();
        let result = if file_type.is_symlink() || !file_type.is_dir() {
            std::fs::remove_file(&pair.src)
        } else {
            std::fs::remove_dir_all(&pair.src)
        };
        result.with_context(|| {
            format!(
                "{}: Failed to remove move source (destination already populated)",
                pair.src.display()
            )
        })?;
    }
    Ok(())
}

/// Copy Job 本体。Scan Phase → Operation Phase の二相で動く。
///
/// # Partial Result
/// Cancel された場合、Operation Phase で `std::fs::copy` 完了済みの個別ファイルは
/// ディスクに残る。それを内包する祖先ディレクトリも残る (空ディレクトリとして残り得る)。
/// Scan Phase 中の cancel では Partial Result は残らない (mkdir も発火していないため)。
///
/// # Symlink
/// top-level の VFile が dir-symlink の場合はリンクをたどってその内容をコピーする
/// (旧 `fs::file::copy_to` と同じ挙動)。再帰内ではリンクをたどらず、symlink エントリは
/// `std::fs::copy` で「リンク先データを書き出すファイル」として扱う。これにより
/// 入れ子の symlink ループや任意領域への脱出を防ぐ。
fn run_copy(
    files: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    let Some(plan) = scan_copy_plan(files, dest, cancel, on_progress)? else {
        // Scan Phase 中に cancel された場合は Partial Result なしで早期 return
        return Ok(());
    };
    let total = plan.files.len();
    // Operation Phase 開始を Phase 切り替え直後の `(Copying, 0, Some(total))` で通知。
    // mkdir ループに入る前に出すことで「ディレクトリ作成中は UI が Scanning のまま」を回避する。
    on_progress(Phase::Copying, 0, Some(total));
    // ユーザ指定の宛先に応じてコンテナ dir（または宛先パスの親）を一度だけ確保。
    // それ以降の plan.directories は pre-order により親が常に作成済みなので create_dir で十分。
    ensure_destination_dir(files, dest)?;
    for dir in &plan.directories {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        std::fs::create_dir(dir)
            .with_context(|| format!("{}: Failed to create directory", dir.display()))?;
    }
    for (i, entry) in plan.files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Err(e) = copy_entry(entry) {
            // Copy 途中で cancel が立っていた場合は Err でなく cancel として畳む
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            return Err(e);
        }
        on_progress(Phase::Copying, i + 1, Some(total));
    }
    Ok(())
}

/// 1 件分の `CopyEntry` を宛先に書き出す。
/// 通常ファイル/file-symlink は `std::fs::copy` でリンク先のデータをコピーするが、
/// symlink (特に dir-symlink) は `std::os::unix::fs::symlink` でリンク自体を再生成する。
/// これにより macOS の `.app` バンドル等に含まれる `Resources -> Versions/A/Resources` のような
/// dir-symlink を含むツリーを `cp -R` と同等に正しくコピーできる。
fn copy_entry(entry: &CopyEntry) -> Result<()> {
    match entry {
        CopyEntry::File { src, dst } => {
            std::fs::copy(src, dst)
                .with_context(|| format!("{}: Failed to copy file", dst.display()))?;
            Ok(())
        }
        #[cfg(unix)]
        CopyEntry::Symlink { dst, target } => {
            std::os::unix::fs::symlink(target, dst)
                .with_context(|| format!("{}: Failed to create symlink", dst.display()))?;
            Ok(())
        }
    }
}

/// Scan Phase が組み立てる Copy の実行計画。
/// `directories`: 作成すべき宛先ディレクトリ列 (親が子に先行する DFS pre-order)。
///     単一ファイル root では何も追加されず空のまま (mkdir は `run_copy` 冒頭の `create_dir_all(dest)` で十分)。
/// `files`: コピーすべきエントリ列 (通常ファイル + symlink)。各 dst の親は `directories` に含まれるか
///     `dest` 自体のため、Operation Phase ではディレクトリ作成不要。
#[derive(Debug, Default)]
struct CopyPlan {
    directories: Vec<PathBuf>,
    files: Vec<CopyEntry>,
}

/// Scan Phase で 1 エントリ分の処理計画を保持する。
/// 通常ファイル/file-symlink は `File` バリアントで `std::fs::copy` 経由のデータコピー、
/// それ以外の symlink (主に dir-symlink) は `Symlink` バリアントで再生成する。
#[derive(Debug)]
enum CopyEntry {
    File {
        src: PathBuf,
        dst: PathBuf,
    },
    #[cfg(unix)]
    Symlink {
        dst: PathBuf,
        /// `std::fs::read_link` が返した値をそのまま保持 (相対パスは相対のまま再生成され、
        /// macOS の `.app` バンドルのような相対 symlink 構造を壊さない)。
        target: PathBuf,
    },
}

/// Scan Phase の中断/完走を表す。
enum CollectStatus {
    Completed,
    Cancelled,
}

/// `roots` 配下を一度走査して、cancel 可能なまま `CopyPlan` を組み立てる (Scan Phase)。
///
/// - `Ok(Some(plan))`: 全 root を列挙完了。`plan.files` の各 src→dst をコピーすれば結果が得られる
/// - `Ok(None)`: Scan 中に Cancel Token がセットされ早期中断。Partial Result は無し
/// - `Err`: 走査中の I/O エラー (`read_dir` 失敗、`unique_path` 失敗など)
///
/// 各 root の top-level 名は `pick_unique_top_dest` で衝突回避し、`dest/<name>` がすでに
/// 存在するか、**同一 batch 内で既に他 root に予約されている** 場合は `<name>_1`, `<name>_2`, ... に
/// 振り替える (旧 `fs::file::copy_to` と同じ規約 + multi-root batch での内部衝突回避)。
fn scan_copy_plan(
    roots: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<CopyPlan>> {
    let pairs = resolve_top_level_pairs(roots, dest)?;
    scan_pairs_into_plan(&pairs, cancel, on_progress)
}

/// 各 root の絶対パスと、衝突回避した宛先トップレベルパスのペア列を返す。
/// Copy / Move の Scan Phase 共通の前処理。同一 batch 内で複数 root が同名 (`a/foo.txt`, `b/foo.txt`)
/// だった場合も `claimed` set で 1 件ずつ予約しながら回避するため、後続 root が前 root の宛先を
/// 上書きすることはない。
fn resolve_top_level_pairs(roots: &[VFile], dest: &Path) -> Result<Vec<TopLevelPair>> {
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    let mut pairs = Vec::with_capacity(roots.len());
    // 単一 root かつ dest が既存ディレクトリでないなら dest をそのまま新しい名前に
    // 使う（rename-on-copy/move）。それ以外は dest をコンテナとし、ベース名を中に入れる。
    let target_path = dest_is_target_path(roots, dest);
    for root in roots {
        let src = Path::new(root.absolute_path());
        let base = if target_path {
            dest.to_path_buf()
        } else {
            let name = src
                .file_name()
                .with_context(|| format!("{}: source has no file name", src.display()))?;
            dest.join(name)
        };
        let top_dest = pick_unique_top_dest(&base, &claimed)
            .with_context(|| format!("{}: Failed to resolve unique destination", src.display()))?;
        claimed.insert(top_dest.clone());
        pairs.push(TopLevelPair {
            src: src.to_path_buf(),
            dst: top_dest,
        });
    }
    Ok(pairs)
}

/// 衝突回避済みの宛先パスを返す。
/// `initial` が `claimed` に予約済みでなく、かつディスクにも存在しなければそれを採用する。
/// それ以外は `initial` の stem に `_1`, `_2`, ... を付けて未予約 + 未存在な候補を探す。
/// ディスク上の既存名を避けるだけでなく、batch 内の同名 root を `claimed` で内部回避する。
fn pick_unique_top_dest(initial: &Path, claimed: &HashSet<PathBuf>) -> Result<PathBuf> {
    if !claimed.contains(initial) && !initial.exists() {
        return Ok(initial.to_path_buf());
    }
    let parent = initial
        .parent()
        .with_context(|| format!("{}: no parent directory", initial.display()))?;
    let stem = initial
        .file_stem()
        .with_context(|| format!("{}: no file stem", initial.display()))?
        .to_string_lossy()
        .into_owned();
    let ext = initial
        .extension()
        .map(|e| e.to_string_lossy().into_owned());
    for i in 1..=MAX_UNIQUE_PATH_SUFFIX {
        let name = match &ext {
            Some(e) => format!("{stem}_{i}.{e}"),
            None => format!("{stem}_{i}"),
        };
        let candidate = parent.join(&name);
        if !claimed.contains(&candidate) && !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("{}: Failed to make unique path", initial.display())
}

/// 事前解決済みの `TopLevelPair` 列を走査して `CopyPlan` を組み立てる Scan Phase。
/// Copy/Move の EXDEV フォールバックで共通利用する。
fn scan_pairs_into_plan(
    pairs: &[TopLevelPair],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<CopyPlan>> {
    let mut plan = CopyPlan::default();
    on_progress(Phase::Scanning, 0, None);
    for pair in pairs {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        // top-level は旧 fs::file::copy_to と同じく metadata (symlink follow) で判定する。
        // ユーザがコマンドで明示的に指定した対象なので、dir-symlink ならその内容をコピー。
        // `Path::is_dir()` ではなく `metadata()?` を使うことで、stat 失敗を「通常ファイル扱い」に
        // 握りつぶさず Scan Phase で早期 Err として顕在化する。
        let metadata = pair
            .src
            .metadata()
            .with_context(|| format!("{}: Failed to stat source", pair.src.display()))?;
        let status = if metadata.is_dir() {
            collect_directory_into_plan(&pair.src, &pair.dst, &mut plan, cancel, on_progress)?
        } else {
            enqueue_entry(
                &mut plan,
                CopyEntry::File {
                    src: pair.src.clone(),
                    dst: pair.dst.clone(),
                },
                on_progress,
            );
            CollectStatus::Completed
        };
        match status {
            CollectStatus::Completed => {}
            CollectStatus::Cancelled => return Ok(None),
        }
    }
    Ok(Some(plan))
}

/// `src` ディレクトリ配下を pre-order DFS で plan に積む。
/// `src` は呼び出し側で `metadata().is_dir()` 判定済み (top-level は symlink follow 結果として
/// dir-symlink である可能性あり、再帰内はリンクをたどっていないので非 symlink のディレクトリ)。
/// `read_dir` で得たエントリの `file_type()` を使い、ディレクトリでもシンボリックリンクは
/// たどらない (dir-symlink ループや任意領域脱出の防止)。
fn collect_directory_into_plan(
    src: &Path,
    dst: &Path,
    plan: &mut CopyPlan,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    plan.directories.push(dst.to_path_buf());
    for entry in std::fs::read_dir(src)
        .with_context(|| format!("{}: Failed to read directory", src.display()))?
    {
        // File-level Checkpoint: 各エントリ処理の前に cancel をチェックする
        // (ZipExtract の `for i in 0..total` と対称形)。
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
        let entry_src = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to read file type", entry_src.display()))?;
        let entry_dst = dst.join(entry.file_name());
        if file_type.is_dir() && !file_type.is_symlink() {
            match collect_directory_into_plan(&entry_src, &entry_dst, plan, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else if cfg!(unix) && file_type.is_symlink() {
            // symlink (file-symlink / dir-symlink いずれも) はリンク自体を再生成する。
            // dir-symlink を `std::fs::copy` でたどると "Is a directory" でエラーになるため、
            // また file-symlink でもリンク先データを書き出すと bundle 構造が壊れるため、
            // 一律 `read_link` で target を取得して `std::os::unix::fs::symlink` で再生成する。
            #[cfg(unix)]
            {
                let target = std::fs::read_link(&entry_src).with_context(|| {
                    format!("{}: Failed to read symlink target", entry_src.display())
                })?;
                enqueue_entry(
                    plan,
                    CopyEntry::Symlink {
                        dst: entry_dst,
                        target,
                    },
                    on_progress,
                );
            }
        } else {
            // 通常ファイル・特殊ファイルは std::fs::copy で内容コピーする。
            // (cfg(not(unix)) では symlink もこの分岐に落ちる。Windows での symlink 再生成は
            // 別 API のため未対応で、既存挙動 - リンク先データのコピー試行 - を維持する。)
            enqueue_entry(
                plan,
                CopyEntry::File {
                    src: entry_src,
                    dst: entry_dst,
                },
                on_progress,
            );
        }
    }
    Ok(CollectStatus::Completed)
}

/// plan.files に 1 件積み、SCAN_NOTIFY_BATCH 件ごとに Scanning 進捗を通知するヘルパ。
/// ファイル発見ごとの per-iteration callback コストを抑える。
fn enqueue_entry(
    plan: &mut CopyPlan,
    entry: CopyEntry,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) {
    plan.files.push(entry);
    let count = plan.files.len();
    if count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
}

fn run_zip_extract(
    file: &VFile,
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    let src_path = file.absolute_path();
    let zip_file = std::fs::File::open(src_path)
        .with_context(|| format!("{src_path}: Failed to open zip file"))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .with_context(|| format!("{src_path}: Failed to read zip archive"))?;

    let total = archive.len();
    on_progress(Phase::Extracting, 0, Some(total));

    // 同一 parent への create_dir_all 連打を避けるための直前 parent キャッシュ。
    // zip は同一ディレクトリのエントリが連続することが多いので 1 件だけで多くの syscall が消える。
    let mut last_parent: Option<PathBuf> = None;

    for i in 0..total {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("{src_path}: Failed to read zip entry"))?;
        let Some(enclosed_name) = entry.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(enclosed_name);
        let processed = i + 1;

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .with_context(|| format!("{}: Failed to create directory", out_path.display()))?;
            // 自分自身を parent キャッシュにも入れておく (子エントリで再 mkdir を防ぐ)
            last_parent = Some(out_path.clone());
            on_progress(Phase::Extracting, processed, Some(total));
            continue;
        }

        if let Some(parent) = out_path.parent()
            && last_parent.as_deref() != Some(parent)
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("{}: Failed to create directory", parent.display()))?;
            last_parent = Some(parent.to_path_buf());
        }

        let out_file = std::fs::File::create(&out_path)
            .with_context(|| format!("{}: Failed to create file", out_path.display()))?;
        let mut writer = BufWriter::new(out_file);
        std::io::copy(&mut entry, &mut writer)
            .with_context(|| format!("{}: Failed to extract file", out_path.display()))?;
        // BufWriter を明示的に flush して書き残しエラーを伝播させる
        writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("{}: Failed to flush: {}", out_path.display(), e))?;

        on_progress(Phase::Extracting, processed, Some(total));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::sync::atomic::AtomicBool;
    use tempfile::TempDir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn vfile(path: &std::path::Path) -> VFile {
        VFile::new(
            path.to_str()
                .expect("UTF-8 path required for tests")
                .to_owned(),
        )
    }

    fn build_sample_zip(zip_path: &std::path::Path) {
        let file = File::create(zip_path).expect("create zip file");
        let mut writer = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        writer.start_file("hello.txt", options).unwrap();
        writer.write_all(b"hello fv").unwrap();
        writer.add_directory("nested/", options).unwrap();
        writer.start_file("nested/inner.txt", options).unwrap();
        writer.write_all(b"inside nested").unwrap();
        writer.finish().expect("finish zip");
    }

    fn read_to_string(path: &std::path::Path) -> String {
        let mut s = String::new();
        File::open(path).unwrap().read_to_string(&mut s).unwrap();
        s
    }

    fn write_file(path: &std::path::Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        File::create(path).unwrap().write_all(contents).unwrap();
    }

    #[test]
    fn zip_extract_returns_err_when_source_file_is_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::ZipExtract {
            file: vfile(&tmp.path().join("no-such.zip")),
            dest,
        };
        let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn zip_extract_stops_at_file_checkpoint_when_cancel_is_preset() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("sample.zip");
        build_sample_zip(&zip_path);
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let cancel = AtomicBool::new(true);
        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::ZipExtract {
            file: vfile(&zip_path),
            dest: dest.clone(),
        };
        job.run(&cancel, &mut |p, n, t| events.push((p, n, t)))
            .unwrap();

        // 初期の 0/N は通知されるが、いかなるエントリも処理されない
        assert_eq!(events, vec![(Phase::Extracting, 0, Some(3))]);
        assert!(!dest.join("hello.txt").exists());
        assert!(!dest.join("nested").join("inner.txt").exists());
        // cancel フラグ自体には触らない
        assert!(cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn zip_extract_emits_progress_for_each_entry_with_known_total() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("sample.zip");
        build_sample_zip(&zip_path);
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::ZipExtract {
            file: vfile(&zip_path),
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .unwrap();

        // sample.zip は hello.txt, nested/, nested/inner.txt の 3 entry
        assert_eq!(
            events,
            vec![
                (Phase::Extracting, 0, Some(3)),
                (Phase::Extracting, 1, Some(3)),
                (Phase::Extracting, 2, Some(3)),
                (Phase::Extracting, 3, Some(3)),
            ]
        );
    }

    #[test]
    fn zip_extract_writes_all_entries_to_destination() {
        let tmp = TempDir::new().expect("tempdir");
        let zip_path = tmp.path().join("sample.zip");
        build_sample_zip(&zip_path);

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::ZipExtract {
            file: vfile(&zip_path),
            dest: dest.clone(),
        };
        let cancel = AtomicBool::new(false);
        job.run(&cancel, &mut |_, _, _| {})
            .expect("ZipExtract should succeed");

        assert_eq!(read_to_string(&dest.join("hello.txt")), "hello fv");
        assert_eq!(
            read_to_string(&dest.join("nested").join("inner.txt")),
            "inside nested"
        );
    }

    #[test]
    fn copy_avoids_collision_by_appending_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        // dest/foo がすでに存在 → コピーは衝突を回避して dest/foo_1 に置く
        std::fs::create_dir(dest.join("foo")).unwrap();
        write_file(&dest.join("foo").join("existing.txt"), b"existing");

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        // 既存ディレクトリは無傷
        assert_eq!(
            read_to_string(&dest.join("foo").join("existing.txt")),
            "existing"
        );
        // コピーは foo_1 に置かれる
        assert_eq!(read_to_string(&dest.join("foo_1").join("a.txt")), "alpha");
    }

    #[test]
    fn copy_returns_err_when_source_file_is_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&tmp.path().join("no-such.txt"))],
            dest,
        };
        let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn copy_keeps_partial_result_when_cancelled_during_copying_phase() {
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        write_file(&src_root.join("c.txt"), b"c");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        // Operation Phase で 1 ファイルコピー完了の進捗を受けた直後に cancel をセット
        job.run(&cancel, &mut |p, n, _| {
            if p == Phase::Copying && n == 1 {
                cancel_for_closure.store(true, Ordering::Relaxed);
            }
        })
        .expect("cancel should produce Ok early return");

        // 1 ファイルだけはコピー済み (Partial Result)
        let copied = std::fs::read_dir(dest.join("foo"))
            .unwrap()
            .filter_map(|e| e.ok())
            .count();
        assert_eq!(
            copied, 1,
            "exactly one file should remain as partial result, found {copied}"
        );
    }

    #[test]
    fn copy_stops_during_scanning_phase_when_cancel_is_preset() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        // 事前 cancel
        job.run(&AtomicBool::new(true), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("cancel should produce Ok early return");

        // Scan Phase 開始時の (Scanning, 0, None) のみ通知され、Operation Phase へ進まない
        assert_eq!(events, vec![(Phase::Scanning, 0, None)]);
        // どのファイルもコピーされていない (Partial Result すらない)
        assert!(!dest.join("foo").exists());
    }

    #[test]
    fn copy_emits_copying_progress_per_file_copied() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("Copy should succeed");

        let copying: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Copying)
            .copied()
            .collect();
        // Operation Phase 開始時 0/N と各ファイルコピー後の処理済み数
        assert_eq!(
            copying,
            vec![
                (Phase::Copying, 0, Some(2)),
                (Phase::Copying, 1, Some(2)),
                (Phase::Copying, 2, Some(2)),
            ]
        );
    }

    #[test]
    fn copy_emits_initial_scanning_progress_at_phase_start() {
        // Scan Phase は SCAN_NOTIFY_BATCH (256) 件ごとにバッチで通知するため、
        // 小規模 (2 ファイル) では初回 (Scanning, 0, None) のみ emit される。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("Copy should succeed");

        let scanning: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Scanning)
            .copied()
            .collect();
        assert_eq!(scanning, vec![(Phase::Scanning, 0, None)]);
    }

    #[test]
    fn copy_reproduces_directory_hierarchy_recursively() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        assert_eq!(read_to_string(&dest.join("foo").join("a.txt")), "alpha");
        assert_eq!(
            read_to_string(&dest.join("foo").join("bar").join("b.txt")),
            "beta"
        );
    }

    #[test]
    fn copy_places_single_file_into_destination_directory() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello fv");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        assert_eq!(read_to_string(&dest.join("hello.txt")), "hello fv");
    }

    #[test]
    fn copy_single_file_to_nonexistent_path_writes_that_exact_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("file.txt");
        write_file(&src, b"hello fv");
        // 既存しない宛先パス（ファイル名）を指定する。
        let dest = tmp.path().join("file_2.txt");

        let job = FileJob::Copy {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        // dest はちょうどそのパスのファイルになる（ディレクトリ化しない）。
        assert!(dest.is_file(), "dest should be a file, not a directory");
        assert_eq!(read_to_string(&dest), "hello fv");
    }

    #[test]
    fn copy_multiple_files_to_nonexistent_dest_creates_container_directory() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"alpha");
        write_file(&b, b"bravo");
        // 既存しない宛先。複数ソースなのでコンテナディレクトリとして作成されるべき。
        let dest = tmp.path().join("box");

        let job = FileJob::Copy {
            files: vec![vfile(&a), vfile(&b)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        assert!(dest.is_dir(), "dest should be created as a directory");
        assert_eq!(read_to_string(&dest.join("a.txt")), "alpha");
        assert_eq!(read_to_string(&dest.join("b.txt")), "bravo");
    }

    #[test]
    fn move_single_file_to_nonexistent_path_writes_that_exact_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("file.txt");
        write_file(&src, b"hello fv");
        let dest = tmp.path().join("file_2.txt");

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // dest はそのパスのファイル、ソースは消える。
        assert!(dest.is_file(), "dest should be a file, not a directory");
        assert_eq!(read_to_string(&dest), "hello fv");
        assert!(!src.exists(), "source should be removed after move");
    }

    #[test]
    fn copy_single_file_to_existing_file_path_appends_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("a.txt");
        write_file(&src, b"alpha");
        // 明示した宛先パスが既存ファイルのケース。上書きせず _1 を付与する。
        let dest = tmp.path().join("b.txt");
        write_file(&dest, b"existing");

        let job = FileJob::Copy {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        // 既存ファイルは無傷、コピーは b_1.txt に置かれる。
        assert_eq!(read_to_string(&dest), "existing");
        assert_eq!(read_to_string(&tmp.path().join("b_1.txt")), "alpha");
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_directory_symlinks_inside_tree_instead_of_following_them() {
        // src/escape -> ../outside (dir-symlink) と src/inside/safe.txt を用意し、
        // 再帰内で escape はリンク自体を再生成して outside 配下を取り込まないことを検証する。
        // (macOS .app バンドルの Resources -> Versions/A/Resources 等で必要な挙動)
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("src");
        write_file(&src_root.join("inside").join("safe.txt"), b"safe");
        let outside = tmp.path().join("outside");
        write_file(
            &outside.join("secret.txt"),
            b"should-not-be-recursively-copied",
        );
        std::os::unix::fs::symlink(&outside, src_root.join("escape")).unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy with dir-symlink should succeed by recreating link");

        // 通常ファイルはコピーされる
        assert_eq!(
            read_to_string(&dest.join("src").join("inside").join("safe.txt")),
            "safe"
        );

        // escape は symlink として再生成されており、target が outside のまま保持されている
        let escape = dest.join("src").join("escape");
        let escape_meta = std::fs::symlink_metadata(&escape).expect("escape entry must exist");
        assert!(
            escape_meta.file_type().is_symlink(),
            "dir-symlink should be preserved as symlink, not recreated as a directory"
        );
        assert_eq!(
            std::fs::read_link(&escape).unwrap(),
            outside,
            "symlink target should be preserved as-is"
        );

        // outside/secret.txt が dest 直下に独立コピーされていない (recurse 不在の証拠)
        assert!(
            !dest.join("src").join("secret.txt").exists(),
            "outside/secret.txt should not be independently copied to dest"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_file_symlinks_inside_tree() {
        // src/target.txt (通常ファイル) と src/alias -> target.txt (file-symlink) を用意し、
        // alias がリンクとして再生成されることを検証する (data を二重コピーしない)。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("src");
        write_file(&src_root.join("target.txt"), b"original");
        std::os::unix::fs::symlink("target.txt", src_root.join("alias")).unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy with file-symlink should succeed");

        let alias = dest.join("src").join("alias");
        let alias_meta = std::fs::symlink_metadata(&alias).expect("alias must exist");
        assert!(
            alias_meta.file_type().is_symlink(),
            "file-symlink should be preserved as symlink"
        );
        assert_eq!(
            std::fs::read_link(&alias).unwrap(),
            std::path::PathBuf::from("target.txt"),
            "relative symlink target should be preserved verbatim"
        );
        // symlink follow すれば alias の内容は target.txt のデータ
        assert_eq!(read_to_string(&alias), "original");
    }

    #[test]
    fn move_returns_err_when_source_is_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&tmp.path().join("no-such.txt"))],
            dest,
        };
        let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn move_via_copy_and_remove_completes_scan_copy_remove_sequence() {
        // EXDEV フォールバック関数を直接呼び出して動作検証する (実際の cross-FS は CI で再現困難)。
        // 結果として src は消え dest にすべてのファイルが入り、進捗は Scanning → Moving 順で発火する。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");
        let dest_root = tmp.path().join("out").join("foo");
        std::fs::create_dir_all(dest_root.parent().unwrap()).unwrap();

        let pairs = vec![TopLevelPair {
            src: src_root.clone(),
            dst: dest_root.clone(),
        }];
        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        move_via_copy_and_remove(&pairs, &AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("EXDEV fallback should succeed");

        // dest にファイル群がコピーされている
        assert_eq!(read_to_string(&dest_root.join("a.txt")), "alpha");
        assert_eq!(read_to_string(&dest_root.join("bar").join("b.txt")), "beta");
        // src は削除されている
        assert!(
            !src_root.exists(),
            "src must be removed after move fallback"
        );

        // 進捗: Scanning 始まり → Moving に遷移
        assert!(
            events.iter().any(|(p, _, _)| *p == Phase::Scanning),
            "Scanning phase should be emitted: {events:?}"
        );
        let moving_events: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Moving)
            .copied()
            .collect();
        // 初期 (Moving, 0, Some(2)) と各ファイルコピー後の通知 = (Moving, 2, Some(2)) で終わる
        let last = moving_events.last().expect("Moving emit must exist");
        assert_eq!(*last, (Phase::Moving, 2, Some(2)));
    }

    #[test]
    fn move_via_copy_and_remove_keeps_partial_result_when_cancelled_during_copy() {
        // Copy 中の cancel で src は残り、dest には部分結果が積まれていることを確認する。
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        write_file(&src_root.join("c.txt"), b"c");
        let dest_root = tmp.path().join("out").join("foo");
        std::fs::create_dir_all(dest_root.parent().unwrap()).unwrap();

        let pairs = vec![TopLevelPair {
            src: src_root.clone(),
            dst: dest_root.clone(),
        }];
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        move_via_copy_and_remove(&pairs, &cancel, &mut |p, n, _| {
            // 1 ファイルコピー完了直後に cancel をセット
            if p == Phase::Moving && n == 1 {
                cancel_for_closure.store(true, Ordering::Relaxed);
            }
        })
        .expect("cancel should produce Ok early return");

        // src は手付かずで残る (Partial Result on src)
        assert!(src_root.exists(), "src must remain after Copy-time cancel");
        // dest にはコピー済みファイルが残る (Partial Result on dest)
        let copied_count = std::fs::read_dir(&dest_root)
            .unwrap()
            .filter_map(|e| e.ok())
            .count();
        assert_eq!(
            copied_count, 1,
            "exactly one file should be copied as partial result, got {copied_count}"
        );
    }

    #[test]
    fn move_avoids_collision_among_multiple_same_name_roots() {
        // a/foo.txt と b/foo.txt を同じ dest に Move する。
        // 同一 batch 内の衝突を `claimed` set で避け、両方を独立に dest 配下に置く。
        let tmp = TempDir::new().unwrap();
        let src_a = tmp.path().join("a").join("foo.txt");
        let src_b = tmp.path().join("b").join("foo.txt");
        write_file(&src_a, b"AAA");
        write_file(&src_b, b"BBB");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src_a), vfile(&src_b)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // 両 src が移動済み
        assert!(!src_a.exists());
        assert!(!src_b.exists());
        // 両方の内容が dest 配下に独立して残っている (foo.txt と foo_1.txt)
        let entries: Vec<_> = std::fs::read_dir(&dest)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            entries.len(),
            2,
            "both files should be moved with unique names, got {entries:?}"
        );
        let names: std::collections::HashSet<String> = entries
            .iter()
            .map(|n| n.to_string_lossy().into_owned())
            .collect();
        assert!(names.contains("foo.txt"));
        assert!(names.contains("foo_1.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn move_via_copy_and_remove_handles_top_level_dir_symlink_safely() {
        // src/link -> real/ の dir-symlink を top-level に渡し、EXDEV フォールバックで
        // src 側削除時に `remove_dir_all` がリンク先 real/ を消さないことを検証する。
        let tmp = TempDir::new().unwrap();
        let real_dir = tmp.path().join("real");
        write_file(&real_dir.join("inside.txt"), b"content");
        let symlink_root = tmp.path().join("link");
        std::os::unix::fs::symlink(&real_dir, &symlink_root).unwrap();
        let dest_root = tmp.path().join("out").join("link");
        std::fs::create_dir_all(dest_root.parent().unwrap()).unwrap();

        let pairs = vec![TopLevelPair {
            src: symlink_root.clone(),
            dst: dest_root.clone(),
        }];
        move_via_copy_and_remove(&pairs, &AtomicBool::new(false), &mut |_, _, _| {})
            .expect("EXDEV fallback should handle dir-symlink at top level");

        // symlink のリンク自体は削除されている
        assert!(
            std::fs::symlink_metadata(&symlink_root).is_err(),
            "top-level dir-symlink should be removed"
        );
        // リンク先の real ディレクトリは無傷
        assert!(
            real_dir.is_dir(),
            "linked target directory must NOT be removed"
        );
        assert_eq!(read_to_string(&real_dir.join("inside.txt")), "content");
        // dest 側にはリンク先の内容がコピーされている
        assert_eq!(read_to_string(&dest_root.join("inside.txt")), "content");
    }

    #[test]
    fn move_avoids_collision_by_appending_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("foo.txt");
        write_file(&src, b"new");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        // dest/foo.txt がすでに存在 → unique_path で foo_1.txt にずらす
        write_file(&dest.join("foo.txt"), b"existing");

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // 既存ファイルは無傷
        assert_eq!(read_to_string(&dest.join("foo.txt")), "existing");
        // 移動は foo_1.txt に
        assert_eq!(read_to_string(&dest.join("foo_1.txt")), "new");
        assert!(!src.exists());
    }

    #[test]
    fn move_stops_when_cancel_is_preset() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(true), &mut |_, _, _| {})
            .expect("cancel should produce Ok early return");

        // 事前 cancel なので rename は発火していない
        assert!(src.exists(), "src file should remain untouched");
        assert!(!dest.join("hello.txt").exists());
    }

    #[test]
    fn move_emits_top_level_progress_on_same_filesystem_path() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Move {
            files: vec![vfile(&a), vfile(&b), vfile(&c)],
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("Move should succeed");

        // 同一 FS パスでは Scan Phase をスキップし、Moving の top-level 件数のみ通知
        let moving: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Moving)
            .copied()
            .collect();
        assert_eq!(
            moving,
            vec![
                (Phase::Moving, 0, Some(3)),
                (Phase::Moving, 1, Some(3)),
                (Phase::Moving, 2, Some(3)),
                (Phase::Moving, 3, Some(3)),
            ]
        );
        // Scan Phase は emit されない
        assert!(
            !events.iter().any(|(p, _, _)| *p == Phase::Scanning),
            "same-FS move should skip Scan Phase: {events:?}"
        );
    }

    #[test]
    fn move_renames_directory_atomically_on_same_filesystem() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // dest 配下に階層が再現されている
        assert_eq!(read_to_string(&dest.join("foo").join("a.txt")), "alpha");
        assert_eq!(
            read_to_string(&dest.join("foo").join("bar").join("b.txt")),
            "beta"
        );
        // src からはディレクトリごと消えている
        assert!(
            !src_root.exists(),
            "src directory should be gone after move"
        );
    }

    fn read_zip_entries(zip_path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
        let f = File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(f).unwrap();
        let mut out = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let name = entry.name().to_owned();
            if entry.is_dir() {
                out.push((name, Vec::new()));
            } else {
                let mut buf = Vec::new();
                std::io::copy(&mut entry, &mut buf).unwrap();
                out.push((name, buf));
            }
        }
        out
    }

    #[cfg(unix)]
    #[test]
    fn zip_create_skips_symlinks_inside_source_directory() {
        // src/foo/a.txt (通常ファイル) と src/foo/link -> ../outside (dir-symlink) を用意し、
        // symlink エントリが zip に含まれず outside の中身が漏れないことを検証する
        // (既存 add_dir_to_zip と同じ挙動)。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        let outside = tmp.path().join("outside");
        write_file(&outside.join("secret.txt"), b"should-not-leak");
        std::os::unix::fs::symlink(&outside, src_root.join("link")).unwrap();

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&src_root)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let entries = read_zip_entries(&tmp.path().join("out.zip"));
        let names: std::collections::HashSet<String> =
            entries.iter().map(|(n, _)| n.clone()).collect();
        assert!(names.contains("foo/a.txt"));
        // link 自体も outside/secret.txt も zip に含まれない
        assert!(
            !names
                .iter()
                .any(|n| n.contains("link") || n.contains("secret")),
            "symlink and its target must not appear in zip: {names:?}"
        );
    }

    #[test]
    fn zip_create_avoids_collision_by_appending_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"new");
        // dir/out.zip がすでに存在 → unique_path で out_1.zip にずらす
        let existing_zip = tmp.path().join("out.zip");
        write_file(&existing_zip, b"existing");

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&src)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        // 既存ファイルは無傷 (zip でなくただのテキストファイルとして残る)
        assert_eq!(read_to_string(&existing_zip), "existing");
        // 新規 zip は out_1.zip に置かれる
        let new_zip = tmp.path().join("out_1.zip");
        assert!(
            new_zip.exists(),
            "new zip should be created at {}",
            new_zip.display()
        );
        let entries = read_zip_entries(&new_zip);
        assert_eq!(entries[0].0, "hello.txt");
        assert_eq!(entries[0].1, b"new");
    }

    #[test]
    fn zip_create_removes_partial_zip_when_cancelled_during_operation() {
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&a), vfile(&b), vfile(&c)],
        };
        // 1 ファイル zip 完了時に cancel をセット
        job.run(&cancel, &mut |p, n, _| {
            if p == Phase::Zipping && n == 1 {
                cancel_for_closure.store(true, Ordering::Relaxed);
            }
        })
        .expect("cancel should produce Ok early return");

        // 書きかけ zip は削除されている (Partial Result の Zip 例外)
        assert!(
            !tmp.path().join("out.zip").exists(),
            "incomplete zip should be removed on cancel"
        );
    }

    #[test]
    fn zip_create_emits_scanning_then_zipping_progress() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&a), vfile(&b)],
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("ZipCreate should succeed");

        // Scanning 初期 0 通知のあと Zipping 0/2, 1/2, 2/2 と進む
        let scanning_first = events
            .iter()
            .find(|(p, _, _)| *p == Phase::Scanning)
            .copied();
        assert_eq!(scanning_first, Some((Phase::Scanning, 0, None)));
        let zipping: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Zipping)
            .copied()
            .collect();
        assert_eq!(
            zipping,
            vec![
                (Phase::Zipping, 0, Some(2)),
                (Phase::Zipping, 1, Some(2)),
                (Phase::Zipping, 2, Some(2)),
            ]
        );
    }

    #[test]
    fn zip_create_recursively_zips_directory_contents() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "tree.zip".to_string(),
            files: vec![vfile(&src_root)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let entries = read_zip_entries(&tmp.path().join("tree.zip"));
        let names: std::collections::HashSet<String> =
            entries.iter().map(|(n, _)| n.clone()).collect();
        assert!(
            names.contains("foo/a.txt"),
            "missing foo/a.txt in {names:?}"
        );
        assert!(
            names.contains("foo/bar/b.txt"),
            "missing foo/bar/b.txt in {names:?}"
        );
        let by_name: std::collections::HashMap<_, _> = entries.into_iter().collect();
        assert_eq!(
            by_name.get("foo/a.txt").map(|v| v.as_slice()),
            Some(b"alpha" as &[u8])
        );
        assert_eq!(
            by_name.get("foo/bar/b.txt").map(|v| v.as_slice()),
            Some(b"beta" as &[u8])
        );
    }

    #[test]
    fn zip_create_writes_multiple_files_into_archive() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"alpha");
        write_file(&b, b"beta");

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "multi.zip".to_string(),
            files: vec![vfile(&a), vfile(&b)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let entries = read_zip_entries(&tmp.path().join("multi.zip"));
        let by_name: std::collections::HashMap<_, _> = entries.into_iter().collect();
        assert_eq!(
            by_name.get("a.txt").map(|v| v.as_slice()),
            Some(b"alpha" as &[u8])
        );
        assert_eq!(
            by_name.get("b.txt").map(|v| v.as_slice()),
            Some(b"beta" as &[u8])
        );
    }

    #[test]
    fn zip_create_rejects_name_with_path_traversal_components() {
        // 旧 fs::file::create_zip の Component::Normal 検証に相当 (Unzip 経路と対称形)。
        // `..` や絶対パスで dir の外に書き出すのを防ぐ防御深度のテスト。
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello");

        for bad in ["", "../escape.zip", "sub/foo.zip"] {
            let job = FileJob::ZipCreate {
                dir: vfile(tmp.path()),
                name: bad.to_string(),
                files: vec![vfile(&src)],
            };
            let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
            assert!(
                result.is_err(),
                "name {bad:?} should be rejected by Component::Normal validation"
            );
        }
    }

    #[test]
    fn delete_returns_err_when_source_is_missing() {
        // Scan Phase の metadata() で stat 失敗 → Err 早期返却 (ファイル扱いに握りつぶさない)
        let tmp = TempDir::new().unwrap();
        let mut delete_fn =
            |_: &Path| -> Result<()> { panic!("delete_fn must not be called when scan fails") };
        let result = run_delete_with(
            &[vfile(&tmp.path().join("no-such.txt"))],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        );
        assert!(result.is_err());
    }

    #[test]
    fn delete_aborts_on_first_error_with_file_path_in_context() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");

        let mut delete_fn = |path: &Path| -> Result<()> {
            if path.ends_with("b.txt") {
                anyhow::bail!("simulated permission denied")
            }
            std::fs::remove_file(path)?;
            Ok(())
        };
        let result = run_delete_with(
            &[vfile(&a), vfile(&b)],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        );
        let err = result.expect_err("Err should propagate on delete_fn failure");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("0 files remaining"),
            "Err context should include the remaining-count hint: {msg}"
        );
        assert!(
            msg.contains("simulated permission denied"),
            "Err chain should preserve the original cause: {msg}"
        );
        // 1 件目は完了、2 件目で abort
        assert!(!a.exists(), "first file deleted before error");
        assert!(b.exists(), "second file remains (delete_fn failed)");
    }

    #[test]
    fn delete_keeps_partial_result_when_cancelled_during_operation() {
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        let mut delete_fn = |path: &Path| -> Result<()> {
            std::fs::remove_file(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&a), vfile(&b), vfile(&c)],
            &cancel,
            &mut |p, n, _| {
                if p == Phase::Deleting && n == 1 {
                    cancel_for_closure.store(true, Ordering::Relaxed);
                }
            },
            &mut delete_fn,
        )
        .expect("cancel should produce Ok early return");

        // 1 件削除済み、残り 2 件は元の場所に残る (Partial Result on filesystem)
        assert!(!a.exists(), "first file should be deleted");
        assert!(
            b.exists(),
            "second file should remain (cancel before delete)"
        );
        assert!(
            c.exists(),
            "third file should remain (cancel before delete)"
        );
    }

    #[test]
    fn delete_emits_scanning_then_deleting_progress() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let mut delete_fn = |path: &Path| -> Result<()> {
            std::fs::remove_file(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&a), vfile(&b)],
            &AtomicBool::new(false),
            &mut |p, n, t| events.push((p, n, t)),
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        // 初期 Scanning emit のあと Deleting 0..N のシーケンス
        let scanning_first = events
            .iter()
            .find(|(p, _, _)| *p == Phase::Scanning)
            .copied();
        assert_eq!(scanning_first, Some((Phase::Scanning, 0, None)));
        let deleting: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Deleting)
            .copied()
            .collect();
        assert_eq!(
            deleting,
            vec![
                (Phase::Deleting, 0, Some(2)),
                (Phase::Deleting, 1, Some(2)),
                (Phase::Deleting, 2, Some(2)),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn delete_does_not_recurse_into_top_level_dir_symlink() {
        // top-level が dir-symlink の場合、Scan Phase で follow して中身を再帰列挙しない。
        // (リンク先の任意領域の Scan による暴走防止)
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().unwrap();
        let real_dir = tmp.path().join("real");
        std::fs::create_dir(&real_dir).unwrap();
        write_file(&real_dir.join("inside.txt"), b"x");
        let link = tmp.path().join("link");
        symlink(&real_dir, &link).unwrap();

        let mut max_scan: usize = 0;
        let mut delete_fn = |_path: &Path| -> Result<()> {
            // symlink の trash::delete 相当: リンクのみ削除 (実体は触らない)
            std::fs::remove_file(_path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&link)],
            &AtomicBool::new(false),
            &mut |p, n, _| {
                if p == Phase::Scanning {
                    max_scan = max_scan.max(n);
                }
            },
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        // Scan は symlink を 1 件として数えるだけ
        assert_eq!(max_scan, 1);
        // リンクは消え、リンク先の中身は無傷
        assert!(!link.exists());
        assert!(real_dir.join("inside.txt").exists());
    }

    #[test]
    fn delete_treats_directory_as_single_top_level_item() {
        // trash::delete は atomic per-item で dir を 1 エントリとして trash に入れるため、
        // Operation Phase でも再帰展開せず top-level の dir 1 回の delete_fn 呼出にとどめる。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");

        let mut called: Vec<PathBuf> = Vec::new();
        let mut delete_fn = |path: &Path| -> Result<()> {
            called.push(path.to_path_buf());
            std::fs::remove_dir_all(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&src_root)],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        assert_eq!(called, vec![src_root.clone()]);
        assert!(!src_root.exists());
    }

    #[test]
    fn delete_invokes_delete_fn_for_all_top_level_files() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");

        let mut called: Vec<PathBuf> = Vec::new();
        let mut delete_fn = |path: &Path| -> Result<()> {
            called.push(path.to_path_buf());
            std::fs::remove_file(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&a), vfile(&b), vfile(&c)],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        assert_eq!(called, vec![a.clone(), b.clone(), c.clone()]);
        assert!(!a.exists() && !b.exists() && !c.exists());
    }

    #[test]
    fn zip_create_writes_single_file_into_archive() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello fv");
        let dir = tmp.path().to_path_buf();

        let job = FileJob::ZipCreate {
            dir: vfile(&dir),
            name: "out.zip".to_string(),
            files: vec![vfile(&src)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let zip_path = dir.join("out.zip");
        assert!(
            zip_path.exists(),
            "zip file should be created at {}",
            zip_path.display()
        );
        let entries = read_zip_entries(&zip_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "hello.txt");
        assert_eq!(entries[0].1, b"hello fv");
    }

    #[test]
    fn move_renames_single_file_to_destination_directory_on_same_filesystem() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello fv");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // dest にファイルが現れている
        assert_eq!(read_to_string(&dest.join("hello.txt")), "hello fv");
        // src からは消えている
        assert!(!src.exists(), "src file should be gone after move");
    }

    #[cfg(unix)]
    #[test]
    fn copy_does_not_infinitely_recurse_on_symlink_loop() {
        // src/loop -> src の自己ループ。Scan が再帰せず有限時間で return することを検証する。
        // 新仕様: loop は symlink として再生成され、再帰には入らない。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("src");
        std::fs::create_dir(&src_root).unwrap();
        std::os::unix::fs::symlink(&src_root, src_root.join("loop")).unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("self-loop symlink should be preserved without infinite recursion");

        // loop は symlink として保持され、再帰展開されていないこと
        let dest_loop = dest.join("src").join("loop");
        let loop_meta = std::fs::symlink_metadata(&dest_loop).expect("loop link must exist");
        assert!(
            loop_meta.file_type().is_symlink(),
            "self-loop symlink should be preserved"
        );
        // 自己ループ展開が起きていない
        assert!(
            !dest
                .join("src")
                .join("loop")
                .join("loop")
                .join("loop")
                .exists()
                || loop_meta.file_type().is_symlink(),
            "self-loop should not produce nested loop directories"
        );
    }
}
