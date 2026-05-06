use crate::fs::VFileMetadata;
use crate::state::AttributeState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use num_format::{Locale, ToFormattedString};
use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

fn build_rows(metadata: &VFileMetadata) -> Vec<Row<'static>> {
    let file_type = if metadata.is_symlink() {
        "Symlink"
    } else if metadata.is_dir() {
        "Directory"
    } else if metadata.is_file() {
        "File"
    } else {
        "Other"
    };

    let modified = metadata
        .modified()
        .map(|t| t.to_string())
        .unwrap_or_else(|_| "-".to_string());
    let accessed = metadata
        .accessed()
        .map(|t| t.to_string())
        .unwrap_or_else(|_| "-".to_string());
    let created = metadata
        .created()
        .map(|t| t.to_string())
        .unwrap_or_else(|_| "-".to_string());

    let label_style = Style::default().fg(Color::Yellow);

    let mut entries = vec![
        ("File Type", file_type.to_string()),
        (
            "Size",
            format!(
                "{} bytes",
                metadata.file_size().to_formatted_string(&Locale::en)
            ),
        ),
        ("Permissions", metadata.permissions().to_rwx_string()),
    ];

    #[cfg(unix)]
    entries.extend([
        (
            "Mode",
            format!("{:04o}", metadata.mode() & 0o7777),
        ),
        ("Owner (UID)", metadata.uid().to_string()),
        ("Group (GID)", metadata.gid().to_string()),
        ("Hard Links", metadata.nlink().to_string()),
        ("Inode", metadata.ino().to_string()),
        ("Device ID", metadata.dev().to_string()),
        ("Block Size", metadata.blksize().to_string()),
        ("Blocks", metadata.blocks().to_string()),
    ]);

    entries.extend([
        ("Created", created),
        ("Accessed", accessed),
        ("Modified", modified),
    ]);

    debug_assert_eq!(entries.len(), VFileMetadata::attribute_count());

    entries
        .into_iter()
        .map(|(label, value)| {
            Row::new([
                Cell::from(label).style(label_style),
                Cell::from(value),
            ])
        })
        .collect()
}

pub fn build_attribute_table(state: &AttributeState) -> Table<'static> {
    let title = format!("Attribute - {}", state.file_name);
    let block = build_bordered_block(&title, BorderStyle::Active);
    let rows = build_rows(&state.metadata);
    Table::new(rows, [Constraint::Max(14), Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
