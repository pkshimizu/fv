//! ホスト OS／環境との対話（ファイルシステムを超えた領域）。
//! クリップボード書き込みやシステム情報取得など、`fs`（ファイル操作）に属さない
//! ホスト環境とのやり取りをここに集約する。

pub mod clipboard;
pub mod disk_usage;
pub mod system_info;
