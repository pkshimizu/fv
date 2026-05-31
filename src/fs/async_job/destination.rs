//! Copy / Move の **Destination** 解決（CONTEXT.md 参照）。
//! 宛先ディレクトリの確保・「正確な宛先パス」判定・衝突回避済み top-level パスの解決を担う。

use crate::fs::VFile;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 衝突回避時の suffix 探索上限 (`pick_unique_top_dest` で使用)。
const MAX_UNIQUE_PATH_SUFFIX: u32 = 1000;

/// Copy/Move の Scan Phase で扱う「src ファイルパス → 衝突回避済み宛先 top-level パス」のペア。
/// タプルだと `.0/.1` でアクセスする箇所が意図不明になりやすいため struct 化している。
#[derive(Debug, Clone)]
pub(super) struct TopLevelPair {
    pub(super) src: PathBuf,
    pub(super) dst: PathBuf,
}

/// `dest` ディレクトリを必ず存在させる。Copy/Move 入口で共通的に呼び、ユーザが存在しない path を
/// 指定した場合でも先頭で確保する。
fn ensure_dest_dir(dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .with_context(|| format!("{}: Failed to create destination directory", dest.display()))
}

/// dest を「正確な宛先パス」（rename-on-copy/move）として扱うか判定する。
/// 単一 root（= 単一 Operation Target）かつ dest が既存ディレクトリでないときのみ true。
/// false のときは dest をコンテナディレクトリとして各 root をベース名で中に入れる。
/// `dest.is_dir()` の stat を含むため、1 操作につき一度だけ評価して結果を引き回す。
/// CONTEXT.md の Destination 参照。Copy / Move 共通。
pub(super) fn dest_is_exact_path(roots: &[VFile], dest: &Path) -> bool {
    roots.len() == 1 && !dest.is_dir()
}

/// Copy / Move の前に存在を保証すべきディレクトリを作る。
/// コンテナ扱い（`exact_path == false`）なら dest 自身、正確な宛先パス扱いなら dest の親。
/// `exact_path` は呼び出し側で `dest_is_exact_path` を一度だけ評価した結果を渡す。
pub(super) fn ensure_destination_dir(exact_path: bool, dest: &Path) -> Result<()> {
    let dir = if exact_path {
        dest.parent()
    } else {
        Some(dest)
    };
    // 親が無い（ルート直下など）場合は確保すべきディレクトリが無いので何もしない。
    // 実書き込みの失敗は後段の I/O 側 with_context が拾う。
    dir.map_or(Ok(()), ensure_dest_dir)
}

/// 各 root の絶対パスと、衝突回避した宛先トップレベルパスのペア列を返す。
/// Copy / Move の Scan Phase 共通の前処理。同一 batch 内で複数 root が同名 (`a/foo.txt`, `b/foo.txt`)
/// だった場合も `claimed` set で 1 件ずつ予約しながら回避するため、後続 root が前 root の宛先を
/// 上書きすることはない。
pub(super) fn resolve_top_level_pairs(
    roots: &[VFile],
    dest: &Path,
    exact_path: bool,
) -> Result<Vec<TopLevelPair>> {
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    let mut pairs = Vec::with_capacity(roots.len());
    // exact_path のとき dest をそのまま新しい名前に使う（rename-on-copy/move）。
    // それ以外は dest をコンテナとし、各 root のベース名を中に入れる。
    for root in roots {
        let src = Path::new(root.absolute_path());
        let base = if exact_path {
            dest.to_path_buf()
        } else {
            let name = src
                .file_name()
                .with_context(|| format!("{}: Failed to read source file name", src.display()))?;
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
pub(super) fn pick_unique_top_dest(initial: &Path, claimed: &HashSet<PathBuf>) -> Result<PathBuf> {
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
