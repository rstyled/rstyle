//! Complete-key lookup comparison. Prebuild before collecting timing samples.
#[path = "../src/wl/key.rs"]
mod key;
use key::WlTuple;
use std::{
    collections::{BTreeMap, HashMap},
    hash::{BuildHasher, BuildHasherDefault, Hasher},
    hint::black_box,
    time::Instant,
};
use xxhash_rust::xxh3::Xxh3;

struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    }
}

fn keys(count: usize, length: usize, shared_prefix: bool) -> Vec<WlTuple> {
    (0..count)
        .map(|index| {
            let neighbors: Vec<_> = (0..length)
                .map(|step| {
                    let base = if shared_prefix && step + 1 < length {
                        0
                    } else {
                        index * 17
                    };
                    ((step % 3) as u8, ((base + step * 7) % count) as u64)
                })
                .collect();
            WlTuple {
                label: if shared_prefix { 0 } else { (index % 5) as u64 },
                incoming: neighbors.clone(),
                outgoing: neighbors,
            }
        })
        .collect()
}

fn tree(keys: &[WlTuple], entry: bool) -> (Vec<u64>, usize) {
    let mut table = BTreeMap::new();
    let mut labels = Vec::new();
    for key in keys.iter().cycle().take(keys.len() * 3) {
        let tuple = key.clone();
        let next = table.len() as u64;
        let id = if entry {
            *table.entry(tuple).or_insert(next)
        } else if let Some(id) = table.get(&tuple) {
            *id
        } else {
            table.insert(tuple, next);
            next
        };
        labels.push(id);
    }
    (labels, table.len())
}

fn hashed<S: BuildHasher + Default>(keys: &[WlTuple]) -> (Vec<u64>, usize) {
    let mut table = HashMap::<_, _, S>::default();
    let mut labels = Vec::new();
    for key in keys.iter().cycle().take(keys.len() * 3) {
        let next = table.len() as u64;
        labels.push(*table.entry(key.clone()).or_insert(next));
    }
    (labels, table.len())
}

fn measure(name: &str, keys: &[WlTuple], run: impl Fn() -> (Vec<u64>, usize)) {
    let expected = tree(keys, false);
    assert_eq!(run(), expected);
    let seconds: Vec<_> = (0..3)
        .map(|_| {
            let started = Instant::now();
            let actual = black_box(run());
            let elapsed = started.elapsed().as_secs_f64();
            assert_eq!(actual, expected);
            elapsed
        })
        .collect();
    println!(
        "{}",
        serde_json::json!({"algorithm":name, "keys":keys.len(), "neighbors_per_direction":keys[0].incoming.len(), "shared_prefix":keys[0].label == 0 && keys[1].label == 0, "visits":keys.len()*3, "unique":expected.1, "seconds":seconds})
    );
}

fn main() {
    for (count, length) in [(128, 1), (1024, 2), (4096, 4), (1024, 32), (128, 256)] {
        for shared_prefix in [false, true] {
            let keys = keys(count, length, shared_prefix);
            measure("tree-get-insert", &keys, || tree(&keys, false));
            measure("tree-entry", &keys, || tree(&keys, true));
            measure("fnv1a-entry", &keys, || {
                hashed::<BuildHasherDefault<Fnv>>(&keys)
            });
            measure("xxh3-entry", &keys, || {
                hashed::<BuildHasherDefault<Xxh3>>(&keys)
            });
        }
    }
}
