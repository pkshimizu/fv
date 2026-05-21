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
            if Path::new(path).starts_with(Path::new(&child.path)) {
                if let Some(node) = child.find_node_mut(path) {
                    return Some(node);
                }
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

    fn cursor(&mut self) -> TableCursor {
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
            if let Some(parent) = Path::new(&path).parent().and_then(|p| p.to_str()) {
                if let Some(idx) = self
                    .flat_nodes
                    .iter()
                    .position(|e| e.path == parent)
                {
                    self.table_state.select(Some(idx));
                }
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
