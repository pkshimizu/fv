//! テスト共通ヘルパ。各操作モジュールの `#[cfg(test)] mod tests` から利用する。

use crate::fs::VFile;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub(super) fn vfile(path: &Path) -> VFile {
    VFile::new(
        path.to_str()
            .expect("UTF-8 path required for tests")
            .to_owned(),
    )
}

pub(super) fn build_sample_zip(zip_path: &Path) {
    let file = File::create(zip_path).expect("create zip file");
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("hello.txt", options).unwrap();
    writer.write_all(b"hello fv").unwrap();
    writer.add_directory("nested/", options).unwrap();
    writer.start_file("nested/inner.txt", options).unwrap();
    writer.write_all(b"inside nested").unwrap();
    writer.finish().expect("finish zip");
}

pub(super) fn read_to_string(path: &Path) -> String {
    let mut s = String::new();
    File::open(path).unwrap().read_to_string(&mut s).unwrap();
    s
}

pub(super) fn write_file(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    File::create(path).unwrap().write_all(contents).unwrap();
}
