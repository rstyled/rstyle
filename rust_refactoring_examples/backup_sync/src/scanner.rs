use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

pub fn scan_demo_tree(root: &Path) -> Vec<ScannedFile> {
    vec![
        ScannedFile {
            path: root.join("trip/raw-01.bin"),
            contents: b"abcdefghijk".to_vec(),
        },
        ScannedFile {
            path: root.join("trip/notes.txt"),
            contents: b"gate changed to B14".to_vec(),
        },
    ]
}

pub fn to_backup_key(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
