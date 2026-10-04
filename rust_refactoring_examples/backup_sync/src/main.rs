mod checksum;
mod scanner;
mod store;
mod sync;

use std::path::Path;
use store::{MemoryStore, ObjectStore};

#[macro_export]
macro_rules! progress {
    ($($arg:tt)*) => {{
        println!("[backup] {}", format!($($arg)*));
    }};
}

fn main() {
    let files = scanner::scan_demo_tree(Path::new("photos/2026"));
    let mut remote = MemoryStore::default();

    for file in files {
        sync::sync_file(&file, &mut remote).unwrap();
    }

    println!("stored keys: {:?}", remote.list());
}
