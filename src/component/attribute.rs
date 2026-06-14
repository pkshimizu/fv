use crate::component::{Action, Component};
use crate::fs::VFile;
use crate::fs::VFileMetadata;
#[cfg(unix)]
use crate::fs::VPermissions;
use crate::state::table_cursor::TableCursor;
use crate::ui::widgets::build_focused_block;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
#[cfg(unix)]
use ratatui::text::{Line, Span};
#[cfg(unix)]
use ratatui::widgets::Paragraph;
use ratatui::widgets::{Cell, Row, Table, TableState};

/// パーミッションビットのマスク（setuid/setgid/sticky を含む 4 桁）。
#[cfg(unix)]
const PERM_MODE_MASK: u32 = 0o7777;

/// rwx の 1 グループ（user / group / other）あたりのビット数（r/w/x の 3 列）。
/// 編集カーソルのグループ間移動（↑↓）と編集 UI の行レイアウトで共有する。
#[cfg(unix)]
const RWX_GROUP_WIDTH: usize = 3;

/// パーミッション編集（rwx トグル）の作業状態。`draft` は編集中の mode（0o7777）で、
/// 編集 UI に出ない高位ビット（setuid/setgid/sticky）は初期値のまま保持され、適用時も
/// 維持される（rwx 9 ビットのみトグルする）。`cursor` は 0..9 のビット位置。
/// `Enter` で確定、`Esc` で破棄する。
#[cfg(unix)]
struct PermissionEditor {
    draft: u32,
    cursor: usize,
}

pub struct AttributeComponent {
    table_state: TableState,
    file: VFile,
    entries: Vec<(&'static str, String)>,
    /// 編集中のみ `Some`。`None` のときは属性の閲覧モード。
    #[cfg(unix)]
    editor: Option<PermissionEditor>,
    /// パネルが把握している現在の mode（0o7777）。chmod 適用ごとに更新し、再編集時の初期値にする。
    #[cfg(unix)]
    current_mode: u32,
}

impl AttributeComponent {
    pub fn new(file: &VFile) -> Result<Self> {
        let metadata = file.metadata()?;
        let entries = Self::build_entries(metadata);

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Ok(Self {
            table_state,
            #[cfg(unix)]
            current_mode: metadata.mode() & PERM_MODE_MASK,
            file: file.clone(),
            entries,
            #[cfg(unix)]
            editor: None,
        })
    }

    fn build_entries(metadata: &VFileMetadata) -> Vec<(&'static str, String)> {
        let mut entries = Vec::new();
        entries.extend([
            ("File Type", metadata.file_type().to_string()),
            ("Size", metadata.verbose_size()),
            ("Permissions", metadata.permissions().to_rwx_string()),
        ]);

        #[cfg(unix)]
        entries.extend([
            ("Mode", format!("{:04o}", metadata.mode() & PERM_MODE_MASK)),
            ("Owner (UID)", metadata.uid().to_string()),
            ("Group (GID)", metadata.gid().to_string()),
            ("Hard Links", metadata.nlink().to_string()),
            ("Inode", metadata.ino().to_string()),
            ("Device ID", metadata.dev().to_string()),
            ("Block Size", metadata.blksize().to_string()),
            ("Blocks", metadata.blocks().to_string()),
        ]);

        entries.extend([
            (
                "Created",
                metadata
                    .created()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
            (
                "Accessed",
                metadata
                    .accessed()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
            (
                "Modified",
                metadata
                    .modified()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
        ]);
        entries
    }

    fn cursor(&mut self) -> TableCursor<'_> {
        TableCursor::new(&mut self.table_state, self.entries.len())
    }

    /// 選択中の行が "Permissions" か。編集開始の可否と keymap ヒントの出し分けに使う。
    #[cfg(unix)]
    fn is_permissions_row_selected(&self) -> bool {
        self.table_state
            .selected()
            .and_then(|i| self.entries.get(i))
            .is_some_and(|(label, _)| *label == "Permissions")
    }

    /// 選択中の行が "Permissions" なら rwx 編集モードに入る。
    #[cfg(unix)]
    fn try_start_permission_edit(&mut self) {
        if self.is_permissions_row_selected() {
            self.editor = Some(PermissionEditor {
                draft: self.current_mode,
                cursor: 0,
            });
        }
    }

    /// 編集モードのキー処理。Enter で chmod を適用（Action を返す）、Esc で破棄する。
    #[cfg(unix)]
    fn handle_edit_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Left => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.cursor = editor.cursor.saturating_sub(1);
                }
                Ok(Action::None)
            }
            KeyCode::Right => {
                if let Some(editor) = self.editor.as_mut()
                    && editor.cursor + 1 < VPermissions::rwx_bits().len()
                {
                    editor.cursor += 1;
                }
                Ok(Action::None)
            }
            // ↑↓ は user/group/other の行間移動（列＝r/w/x を保ってグループ幅ずつ動く）。
            KeyCode::Up => {
                if let Some(editor) = self.editor.as_mut()
                    && editor.cursor >= RWX_GROUP_WIDTH
                {
                    editor.cursor -= RWX_GROUP_WIDTH;
                }
                Ok(Action::None)
            }
            KeyCode::Down => {
                if let Some(editor) = self.editor.as_mut()
                    && editor.cursor + RWX_GROUP_WIDTH < VPermissions::rwx_bits().len()
                {
                    editor.cursor += RWX_GROUP_WIDTH;
                }
                Ok(Action::None)
            }
            KeyCode::Char(' ') => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.draft ^= VPermissions::rwx_bits()[editor.cursor].0;
                }
                Ok(Action::None)
            }
            KeyCode::Enter => {
                let Some(editor) = self.editor.take() else {
                    return Ok(Action::None);
                };
                let mode = editor.draft;
                self.current_mode = mode;
                self.refresh_permission_entries(mode);
                Ok(Action::SetPermissions(
                    self.file.absolute_path().to_string(),
                    mode,
                ))
            }
            KeyCode::Esc => {
                // 変更を破棄して閲覧モードへ戻る。
                self.editor = None;
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    /// 適用後の mode に合わせて Permissions / Mode 行の表示を更新する。
    /// rwx 文字列は VPermissions に集約された表記を使う（fs 層と単一の真実源）。
    #[cfg(unix)]
    fn refresh_permission_entries(&mut self, mode: u32) {
        let rwx = VPermissions::from_mode(mode).to_rwx_string();
        for entry in &mut self.entries {
            match entry.0 {
                "Permissions" => entry.1 = rwx.clone(),
                "Mode" => entry.1 = format!("{:04o}", mode & PERM_MODE_MASK),
                _ => {}
            }
        }
    }

    /// rwx 編集モードの描画。
    #[cfg(unix)]
    fn render_editor(&self, frame: &mut Frame, area: Rect, draft: u32, cursor: usize) {
        let name = self.file.file_name().unwrap_or("(unknown)");
        let block = build_focused_block(&format!("Edit Permissions - {name}"));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let label_style = Style::default().fg(Color::Yellow);
        let groups = ["user ", "group", "other"];
        let bits = VPermissions::rwx_bits();
        let mut lines: Vec<Line> = Vec::new();
        for (g, group) in groups.iter().enumerate() {
            let mut spans = vec![Span::styled(format!("  {group}: "), label_style)];
            let start = g * RWX_GROUP_WIDTH;
            for (c, &(mask, set_char)) in bits[start..start + RWX_GROUP_WIDTH].iter().enumerate() {
                let i = start + c;
                let ch = if draft & mask != 0 { set_char } else { '-' };
                let style = if i == cursor {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(ch.to_string(), style));
            }
            lines.push(Line::from(spans));
        }
        lines.push(Line::from(format!(
            "  Mode: {:04o}",
            draft & PERM_MODE_MASK
        )));
        frame.render_widget(Paragraph::new(lines), inner);
    }
}

impl Component for AttributeComponent {
    fn keymap(&self) -> &'static str {
        #[cfg(unix)]
        {
            if self.editor.is_some() {
                return "←→: r/w/x  ↑↓: u/g/o  Space: Toggle  Enter: Apply  Esc: Cancel";
            }
            if self.is_permissions_row_selected() {
                return "↑↓: Move  e: Edit permissions  a/Esc: Close";
            }
            "↑↓: Move  a/Esc: Close"
        }
        #[cfg(not(unix))]
        {
            "↑↓: Move  a/Esc: Close"
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        #[cfg(unix)]
        if self.editor.is_some() {
            return self.handle_edit_event(event);
        }
        match event.code {
            KeyCode::Char('a') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            KeyCode::Up => {
                self.cursor().prev();
                Ok(Action::None)
            }
            KeyCode::Down => {
                self.cursor().next();
                Ok(Action::None)
            }
            #[cfg(unix)]
            KeyCode::Char('e') => {
                self.try_start_permission_edit();
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        #[cfg(unix)]
        if let Some(editor) = self.editor.as_ref() {
            let (draft, cursor) = (editor.draft, editor.cursor);
            self.render_editor(frame, area, draft, cursor);
            return;
        }

        let title = format!(
            "Attribute - {}",
            self.file.file_name().unwrap_or("(unknown)")
        );
        let block = build_focused_block(&title);
        let label_style = Style::default().fg(Color::Yellow);
        let rows: Vec<Row> = self
            .entries
            .iter()
            .map(|(label, value)| {
                Row::new([
                    Cell::from(*label).style(label_style),
                    Cell::from(value.as_str()),
                ])
            })
            .collect();
        let table = Table::new(rows, [Constraint::Max(14), Constraint::Fill(1)])
            .block(block)
            .highlight_symbol("> ")
            .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// 指定 mode のファイルを作り、その AttributeComponent を返す。
    fn component_with_mode(mode: u32) -> (TempDir, AttributeComponent) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, "x").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        let file = VFile::new(path.to_str().unwrap());
        let component = AttributeComponent::new(&file).unwrap();
        (tmp, component)
    }

    fn select_permissions_row(c: &mut AttributeComponent) {
        let idx = c
            .entries
            .iter()
            .position(|(l, _)| *l == "Permissions")
            .unwrap();
        c.table_state.select(Some(idx));
    }

    #[test]
    fn e_on_permissions_row_enters_edit_mode() {
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);

        c.handle_event(key(KeyCode::Char('e'))).unwrap();

        assert!(
            c.editor.is_some(),
            "Permissions 行で e を押すと編集モードに入る"
        );
    }

    #[test]
    fn e_on_other_row_does_not_enter_edit_mode() {
        let (_tmp, mut c) = component_with_mode(0o644);
        c.table_state.select(Some(0)); // "File Type" 行

        c.handle_event(key(KeyCode::Char('e'))).unwrap();

        assert!(
            c.editor.is_none(),
            "Permissions 以外の行では編集モードに入らない"
        );
    }

    #[test]
    fn toggle_then_apply_returns_set_permissions_and_updates_display() {
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();

        // cursor=0 は user-r。user-x（index 2）へ移動してトグル → 0o744。
        c.handle_event(key(KeyCode::Right)).unwrap();
        c.handle_event(key(KeyCode::Right)).unwrap();
        c.handle_event(key(KeyCode::Char(' '))).unwrap();
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();

        match action {
            Action::SetPermissions(_, mode) => assert_eq!(mode, 0o744),
            _ => panic!("expected SetPermissions"),
        }
        assert!(c.editor.is_none(), "適用後は閲覧モードへ戻る");
        let mode_entry = c.entries.iter().find(|(l, _)| *l == "Mode").unwrap();
        assert_eq!(mode_entry.1, "0744");
        let perm_entry = c.entries.iter().find(|(l, _)| *l == "Permissions").unwrap();
        assert_eq!(perm_entry.1, "rwxr--r--");
    }

    #[test]
    fn esc_discards_edit() {
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();
        c.handle_event(key(KeyCode::Char(' '))).unwrap(); // user-r をトグル

        let action = c.handle_event(key(KeyCode::Esc)).unwrap();

        assert!(matches!(action, Action::None), "Esc は Action を出さない");
        assert!(c.editor.is_none(), "編集モードを抜ける");
        let mode_entry = c.entries.iter().find(|(l, _)| *l == "Mode").unwrap();
        assert_eq!(mode_entry.1, "0644");
    }

    #[test]
    fn high_bits_setuid_are_preserved_when_toggling_rwx() {
        // setuid 付き 0o4755。user-x（index 2）をトグルして外しても高位ビットは保持される。
        let (_tmp, mut c) = component_with_mode(0o4755);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();

        c.handle_event(key(KeyCode::Right)).unwrap();
        c.handle_event(key(KeyCode::Right)).unwrap();
        c.handle_event(key(KeyCode::Char(' '))).unwrap(); // user-x off
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();

        match action {
            Action::SetPermissions(_, mode) => {
                assert_eq!(mode, 0o4655, "setuid ビットが保持される")
            }
            _ => panic!("expected SetPermissions"),
        }
    }

    #[test]
    fn consecutive_edits_start_from_updated_mode() {
        // 1 回目の編集で 0o644 → 0o744。2 回目の編集はその更新後の値から始まる。
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();
        c.handle_event(key(KeyCode::Right)).unwrap();
        c.handle_event(key(KeyCode::Right)).unwrap();
        c.handle_event(key(KeyCode::Char(' '))).unwrap(); // user-x on → 0o744
        c.handle_event(key(KeyCode::Enter)).unwrap();

        // 2 回目: 何も変えずに Enter すると、更新後の 0o744 がそのまま適用される。
        c.handle_event(key(KeyCode::Char('e'))).unwrap();
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();
        match action {
            Action::SetPermissions(_, mode) => assert_eq!(mode, 0o744),
            _ => panic!("expected SetPermissions"),
        }
    }

    #[test]
    fn down_moves_cursor_to_next_group_same_column() {
        // cursor=0(user-r) から Down で group-r(index 3) へ。0o644 の group-r をトグルすると 0o604。
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();
        c.handle_event(key(KeyCode::Down)).unwrap();
        c.handle_event(key(KeyCode::Char(' '))).unwrap();
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();

        match action {
            Action::SetPermissions(_, mode) => assert_eq!(mode, 0o604),
            _ => panic!("expected SetPermissions"),
        }
    }

    #[test]
    fn up_returns_to_previous_group_same_column() {
        // 0o644。Down→Up で cursor が user-r(0) に戻り（列保持）、user-r をトグル → 0o244。
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();
        c.handle_event(key(KeyCode::Down)).unwrap(); // group-r(3)
        c.handle_event(key(KeyCode::Up)).unwrap(); // user-r(0)
        c.handle_event(key(KeyCode::Char(' '))).unwrap();
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();

        match action {
            Action::SetPermissions(_, mode) => assert_eq!(mode, 0o244),
            _ => panic!("expected SetPermissions"),
        }
    }

    #[test]
    fn up_at_top_group_is_noop() {
        // cursor=0(user-r) で Up しても動かない。user-r をトグル → 0o244。
        let (_tmp, mut c) = component_with_mode(0o644);
        select_permissions_row(&mut c);
        c.handle_event(key(KeyCode::Char('e'))).unwrap();
        c.handle_event(key(KeyCode::Up)).unwrap(); // no-op
        c.handle_event(key(KeyCode::Char(' '))).unwrap();
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();

        match action {
            Action::SetPermissions(_, mode) => assert_eq!(mode, 0o244),
            _ => panic!("expected SetPermissions"),
        }
    }

    #[test]
    fn edit_hint_shown_only_on_permissions_row() {
        let (_tmp, mut c) = component_with_mode(0o644);
        c.table_state.select(Some(0)); // "File Type" 行
        assert!(!c.keymap().contains("Edit permissions"));

        select_permissions_row(&mut c);
        assert!(c.keymap().contains("Edit permissions"));
    }
}
