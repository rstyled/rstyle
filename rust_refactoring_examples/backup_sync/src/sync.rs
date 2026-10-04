use std::path::Path;

use crate::{checksum, progress, scanner::ScannedFile, store::ObjectStore};

fn remote_key_for(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn upload_in_chunks(store: &mut impl ObjectStore, key: &str, bytes: &[u8]) -> Result<(), String> {
    let chunk_size = 4;
    let mut offset = 0;
    let mut assembled = Vec::with_capacity(bytes.len());

    while offset < bytes.len() {
        let end = (offset + chunk_size).min(bytes.len());
        assembled.extend_from_slice(&bytes[offset..end]);
        offset = end;
    }

    store.put(key, &assembled)
}

pub fn sync_file(file: &ScannedFile, store: &mut impl ObjectStore) -> Result<(), String> {
    let base_key = remote_key_for(&file.path);
    let checksum = checksum::rolling_checksum(&file.contents);
    let key = if store.exists(&base_key) {
        format!("{base_key}.conflict-{checksum:x}")
    } else {
        base_key
    };

    progress!("uploading {} as {}", file.path.display(), key);
    upload_in_chunks(store, &key, &file.contents)
}
