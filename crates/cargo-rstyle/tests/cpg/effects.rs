//! The keys Rust's effect table lists, against the definitions the driver gives them to.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Rust's effect table, which core compiles in.
const RUST_TABLE: &str = include_str!("../../../rstyle-core/src/cpg/contracts/effects/rust.toml");

/// Calls every trait implementation the table lists by path.
const KEYED: &str = "use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{ExitCode, Stdio};

pub fn keyed(map: BTreeMap<u8, u8>, set: BTreeSet<u8>, list: Vec<u8>, mut slice: Vec<u8>, text: &str) -> usize {
    for _ in &map {}
    for _ in map.values() {}
    for _ in &set {}
    for _ in &list {}
    for _ in &mut slice {}
    for _ in &list[..] {}
    for _ in &mut slice[..] {}
    for _ in [1u8, 2] {}
    for _ in text.lines() {}
    for _ in text.bytes() {}
    for _ in text.chars() {}
    for _ in Path::new(text).components() {}
    for _ in map {}
    for _ in set {}
    for _ in list {}
    let (flag, word) = (true, 1u64);
    let _ = (ExitCode::from(1u8), Stdio::from(std::io::stderr()));
    text.chars().count() + usize::from(!&flag) + (!&word) as usize + text.to_string().len() + (&text).clone().len()
}
";

/// What each path key names on the pinned nightly, as the document names that definition.
const IMPLEMENTATIONS: &[(&str, &str)] = &[
    (
        "alloc::collections::btree::map::{impl#21}::into_iter",
        "<&'a std::collections::BTreeMap<K, V, A> as std::iter::IntoIterator>::into_iter",
    ),
    (
        "alloc::collections::btree::map::{impl#22}::next",
        "<std::collections::btree_map::Iter<'a, K, V> as std::iter::Iterator>::next",
    ),
    (
        "alloc::collections::btree::map::{impl#38}::next",
        "<std::collections::btree_map::IntoIter<K, V, A> as std::iter::Iterator>::next",
    ),
    (
        "alloc::collections::btree::map::{impl#50}::next",
        "<std::collections::btree_map::Values<'a, K, V> as std::iter::Iterator>::next",
    ),
    (
        "alloc::collections::btree::set::{impl#19}::into_iter",
        "<&'a std::collections::BTreeSet<T, A> as std::iter::IntoIterator>::into_iter",
    ),
    (
        "alloc::collections::btree::set::{impl#32}::next",
        "<std::collections::btree_set::Iter<'a, T> as std::iter::Iterator>::next",
    ),
    (
        "alloc::collections::btree::set::{impl#37}::next",
        "<std::collections::btree_set::IntoIter<T, A> as std::iter::Iterator>::next",
    ),
    (
        "alloc::string::{impl#39}::to_string",
        "<T as std::string::ToString>::to_string",
    ),
    (
        "alloc::vec::into_iter::{impl#6}::next",
        "<std::vec::IntoIter<T, A> as std::iter::Iterator>::next",
    ),
    (
        "alloc::vec::{impl#19}::into_iter",
        "<&'a std::vec::Vec<T, A> as std::iter::IntoIterator>::into_iter",
    ),
    (
        "alloc::vec::{impl#20}::into_iter",
        "<&'a mut std::vec::Vec<T, A> as std::iter::IntoIterator>::into_iter",
    ),
    (
        "core::array::iter::{impl#2}::into_iter",
        "std::array::iter::<impl std::iter::IntoIterator for [T; N]>::into_iter",
    ),
    (
        "core::clone::impls::{impl#6}::clone",
        "std::clone::impls::<impl std::clone::Clone for &T>::clone",
    ),
    (
        "core::iter::traits::collect::{impl#0}::into_iter",
        "<I as std::iter::IntoIterator>::into_iter",
    ),
    (
        "core::ops::bit::{impl#14}::not",
        "<&bool as std::ops::Not>::not",
    ),
    (
        "core::ops::bit::{impl#19}::not",
        "<&u64 as std::ops::Not>::not",
    ),
    (
        "core::slice::iter::{impl#178}::next",
        "<std::slice::IterMut<'a, T> as std::iter::Iterator>::next",
    ),
    (
        "core::slice::iter::{impl#1}::into_iter",
        "core::slice::iter::<impl std::iter::IntoIterator for &'a [T]>::into_iter",
    ),
    (
        "core::slice::iter::{impl#2}::into_iter",
        "core::slice::iter::<impl std::iter::IntoIterator for &'a mut [T]>::into_iter",
    ),
    (
        "core::str::iter::{impl#0}::count",
        "<std::str::Chars<'a> as std::iter::Iterator>::count",
    ),
    (
        "core::str::iter::{impl#0}::next",
        "<std::str::Chars<'a> as std::iter::Iterator>::next",
    ),
    (
        "core::str::iter::{impl#30}::next",
        "<std::str::Lines<'a> as std::iter::Iterator>::next",
    ),
    (
        "core::str::iter::{impl#9}::next",
        "<std::str::Bytes<'_> as std::iter::Iterator>::next",
    ),
    (
        "std::path::{impl#20}::next",
        "<std::path::Components<'a> as std::iter::Iterator>::next",
    ),
    (
        "std::process::{impl#39}::from",
        "<std::process::Stdio as std::convert::From<std::io::Stderr>>::from",
    ),
    (
        "std::process::{impl#54}::from",
        "<std::process::ExitCode as std::convert::From<u8>>::from",
    ),
];

/// The table's exact entries that number an implementation, as a path key does.
fn path_keys(table: &str) -> BTreeSet<&str> {
    table
        .split("{ exact = \"")
        .skip(1)
        .filter_map(|entry| entry.split_once('"').map(|(key, _)| key))
        .filter(|key| key.contains("{impl#"))
        .collect()
}

/// A path key numbers the implementations in their module, so a new nightly can move one onto
/// another implementation of the same member, such as `btree_map::IntoValues`'s `next`, which
/// drops the key. This fails when one moves, which is when the table's paths need checking again.
#[test]
fn the_effect_table_path_keys_name_the_implementations_they_were_checked_against() {
    let listed = path_keys(RUST_TABLE);
    let checked: BTreeSet<&str> = IMPLEMENTATIONS.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        listed, checked,
        "every path key in rust.toml is checked here"
    );

    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), KEYED).unwrap();
    let out = directory.path().join("cpg");
    let emit = rstyle(
        directory.path(),
        &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
    );
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");

    let mut named = BTreeMap::new();
    for path in documents_in(&out) {
        let document: Value = rstyle_core::cpg::transport::read_json(Path::new(&path)).unwrap();
        for definition in document["unit"]["definitions"].as_array().unwrap() {
            if let (Some(key), Some(name)) = (
                definition["operation"].as_str(),
                definition["name"].as_str(),
            ) {
                named.insert(key.to_owned(), name.to_owned());
            }
        }
    }
    for (key, name) in IMPLEMENTATIONS {
        let found = named.get(&format!("rust:{key}"));
        assert_eq!(found.map(String::as_str), Some(*name), "rust:{key}");
    }
}
