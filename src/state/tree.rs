use crate::fs::VFile;
use crate::state::table_cursor::TableCursor;
use ratatui::widgets::TableState;
use std::path::Path;

/// ツリービューの各ノード（ディレクトリまたはファイル）
#[derive(Debug)]
struct TreeNode {
    path: String,
    name: String,
    is_dir: bool,
    expanded: bool,
    children: Vec<TreeNode>,
    children_loaded: bool,
}

impl TreeNode {
    fn new(path: String, name: String, is_dir: bool) -> Self {
        Self {
            path,
            name,
            is_dir,
            expanded: false,
            children: Vec::new(),
            children_loaded: false,
        }
    }

    /// 子ノードをロードする（未ロードの場合のみ）
    fn load_children(&mut self, show_dot_file: bool) {
        if self.children_loaded || !self.is_dir {
            return;
        }
        self.children_loaded = true;
        let Ok(files) = VFile::new(&self.path).list() else {
            return;
        };
        let mut children: Vec<TreeNode> = Vec::new();
        for file in files {
            let Some(name) = file.file_name().map(String::from) else {
                continue;
            };
            // ドットファイルフィルタ
            if !show_dot_file && name.starts_with('.') {
                continue;
            }
            children.push(TreeNode::new(
                file.absolute_path().to_string(),
                name,
                file.is_dir(),
            ));
        }
        // ディレクトリ優先、名前昇順（FilerのSortKeyとは独立。ツリーは常に名前順）
        children.sort_unstable_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        self.children = children;
    }

    /// 指定パスまでの祖先を展開し、子をロードする
    fn expand_to_path(&mut self, target_components: &[&str], depth: usize, show_dot_file: bool) {
        if depth >= target_components.len() {
            return;
        }
        self.load_children(show_dot_file);
        self.expanded = true;
        let target_name = target_components[depth];
        for child in &mut self.children {
            if child.name == target_name {
                child.expand_to_path(target_components, depth + 1, show_dot_file);
                break;
            }
        }
    }

    /// ツリーをフラットリストに変換（深さ優先）
    fn flatten_into(&self, flat: &mut Vec<FlatTreeEntry>, depth: usize) {
        flat.push(FlatTreeEntry {
            path: self.path.clone(),
            name: self.name.clone(),
            depth,
            is_dir: self.is_dir,
            expanded: self.expanded,
        });
        if self.expanded {
            for child in &self.children {
                child.flatten_into(flat, depth + 1);
            }
        }
    }

    /// パスからノードへの可変参照を取得する
    fn find_node_mut(&mut self, path: &str) -> Option<&mut TreeNode> {
        if self.path == path {
            return Some(self);
        }
        for child in &mut self.children {
            if Path::new(path).starts_with(Path::new(&child.path))
                && let Some(node) = child.find_node_mut(path)
            {
                return Some(node);
            }
        }
        None
    }
}

/// フラットリスト上の1エントリ（表示用）
#[derive(Debug)]
pub struct FlatTreeEntry {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

/// ツリービュー全体の状態
#[derive(Debug)]
pub struct TreeState {
    root: TreeNode,
    pub flat_nodes: Vec<FlatTreeEntry>,
    pub table_state: TableState,
    show_dot_file: bool,
}

impl TreeState {
    /// カレントパスを元にツリーを初期化する
    pub fn new(current_path: &str, show_dot_file: bool) -> Self {
        let root_path = "/";
        let mut root = TreeNode::new(root_path.to_string(), "/".to_string(), true);

        // カレントパスまでの祖先を展開
        let path = Path::new(current_path);
        let components: Vec<&str> = path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .collect();
        root.expand_to_path(&components, 0, show_dot_file);

        let mut state = Self {
            root,
            flat_nodes: Vec::new(),
            table_state: TableState::default(),
            show_dot_file,
        };
        state.rebuild_flat_list();

        // カレントパスを選択状態にする
        let select_index = state
            .flat_nodes
            .iter()
            .position(|entry| entry.path == current_path)
            .unwrap_or(0);
        state.table_state.select(Some(select_index));

        state
    }

    fn cursor(&mut self) -> TableCursor<'_> {
        TableCursor::new(&mut self.table_state, self.flat_nodes.len())
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }

    /// 選択中のパスを返す
    pub fn selected_path(&self) -> Option<&str> {
        self.table_state
            .selected()
            .and_then(|i| self.flat_nodes.get(i))
            .map(|entry| entry.path.as_str())
    }

    /// 選択中のエントリ情報を返す
    fn selected_entry(&self) -> Option<&FlatTreeEntry> {
        self.table_state
            .selected()
            .and_then(|i| self.flat_nodes.get(i))
    }

    /// 現在のカーソル位置（Search 開始時に控えて Esc で復元するために使う）。
    pub fn selected_index(&self) -> Option<usize> {
        self.table_state.selected()
    }

    /// カーソル位置を直接設定する（Search の Esc 復元で使う）。
    pub fn select_index(&mut self, index: Option<usize>) {
        self.table_state.select(index);
    }

    /// クエリにマッチする最初のエントリへカーソルを移動する（インクリメンタル検索）。
    pub fn select_matching(&mut self, query: &str) {
        if let Some(i) = self.find_matching_index(query, 0, true) {
            self.table_state.select(Some(i));
        }
    }

    /// 現在位置の次のマッチへ移動する。
    pub fn select_next_matching(&mut self, query: &str) {
        let current = self.table_state.selected().unwrap_or(0);
        if let Some(i) = self.find_matching_index(query, current.wrapping_add(1), true) {
            self.table_state.select(Some(i));
        }
    }

    /// 現在位置の前のマッチへ移動する。
    pub fn select_prev_matching(&mut self, query: &str) {
        let current = self.table_state.selected().unwrap_or(0);
        if let Some(i) = self.find_matching_index(query, current.wrapping_sub(1), false) {
            self.table_state.select(Some(i));
        }
    }

    /// 表示中のノード（flat_nodes）から名前がクエリに部分一致するものを探す。
    /// 展開していないノード配下は flat_nodes に含まれないため、検索対象は
    /// 「ツリーパネルで表示しているエントリのみ」になる。巡回ロジックは Filer と
    /// 共通の `list_search::find_matching_index` に集約している。
    fn find_matching_index(&self, query: &str, start: usize, forward: bool) -> Option<usize> {
        super::list_search::find_matching_index(self.flat_nodes.len(), start, forward, query, |i| {
            Some(self.flat_nodes[i].name.as_str())
        })
    }

    /// 選択中のディレクトリを展開する（Right キー）
    pub fn expand_selected(&mut self) {
        const MAX_TREE_DEPTH: usize = 50;
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if !entry.is_dir || entry.expanded || entry.depth >= MAX_TREE_DEPTH {
            return;
        }
        let path = entry.path.clone();
        let show_dot_file = self.show_dot_file;
        if let Some(node) = self.root.find_node_mut(&path) {
            node.load_children(show_dot_file);
            node.expanded = true;
        }
        self.rebuild_flat_list_preserving_selection(&path);
    }

    /// 選択中のディレクトリを折りたたむ、またはファイル/閉じたディレクトリなら親に移動（Left キー）
    pub fn collapse_selected(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        let is_dir_expanded = entry.is_dir && entry.expanded;
        let path = entry.path.clone();
        if is_dir_expanded {
            if let Some(node) = self.root.find_node_mut(&path) {
                node.expanded = false;
            }
            self.rebuild_flat_list_preserving_selection(&path);
        } else {
            // 親ディレクトリに移動
            if let Some(parent) = Path::new(&path).parent().and_then(|p| p.to_str())
                && let Some(idx) = self.flat_nodes.iter().position(|e| e.path == parent)
            {
                self.table_state.select(Some(idx));
            }
        }
    }

    /// フラットリストを再構築する
    fn rebuild_flat_list(&mut self) {
        self.flat_nodes.clear();
        self.root.flatten_into(&mut self.flat_nodes, 0);
    }

    /// フラットリスト再構築後に指定パスの選択を維持する
    fn rebuild_flat_list_preserving_selection(&mut self, selected_path: &str) {
        self.rebuild_flat_list();
        let idx = self
            .flat_nodes
            .iter()
            .position(|e| e.path == selected_path)
            .unwrap_or(0);
        self.table_state.select(Some(idx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 表示中ノード（flat_nodes）を直接組み立てた TreeState を作る。
    /// 検索は flat_nodes だけを対象とするため、ファイルシステムを介さずに検証できる。
    fn state_with(names: &[&str], selected: usize) -> TreeState {
        let flat_nodes = names
            .iter()
            .map(|n| FlatTreeEntry {
                path: format!("/{n}"),
                name: (*n).to_string(),
                depth: 0,
                is_dir: false,
                expanded: false,
            })
            .collect();
        let mut table_state = TableState::default();
        table_state.select(Some(selected));
        TreeState {
            root: TreeNode::new("/".to_string(), "/".to_string(), true),
            flat_nodes,
            table_state,
            show_dot_file: false,
        }
    }

    #[test]
    fn select_matching_jumps_to_first_case_insensitive_match() {
        let mut state = state_with(&["alpha.rs", "Beta.rs", "gamma.rs"], 0);

        state.select_matching("beta");

        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn select_matching_only_targets_displayed_nodes() {
        // flat_nodes は表示中（展開済み）のエントリのみ。含まれない名前にはマッチせず、
        // カーソルは動かない（＝開いていないノード配下は検索対象にならない）。
        let mut state = state_with(&["src", "README.md"], 0);

        state.select_matching("not_displayed");

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn select_next_and_prev_cycle_through_matches() {
        let mut state = state_with(&["foo1", "bar", "foo2", "foo3"], 0);

        state.select_next_matching("foo");
        assert_eq!(state.selected_index(), Some(2));
        state.select_next_matching("foo");
        assert_eq!(state.selected_index(), Some(3));
        // 末尾の次は先頭の foo へ折り返す。
        state.select_next_matching("foo");
        assert_eq!(state.selected_index(), Some(0));
        // 逆方向は末尾の foo へ。
        state.select_prev_matching("foo");
        assert_eq!(state.selected_index(), Some(3));
    }

    #[test]
    fn select_index_restores_cursor() {
        let mut state = state_with(&["a", "b", "c"], 2);

        state.select_index(Some(0));

        assert_eq!(state.selected_index(), Some(0));
    }
}
