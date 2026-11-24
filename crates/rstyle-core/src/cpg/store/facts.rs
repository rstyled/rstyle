//! The store's function facts: each function's record, kept under a key of everything deriving it
//! reads, so a later run derives only the functions whose key no earlier run kept.
//!
//! A function's key is the sha256 of:
//! - the facts' schema, the rstyle build that derived them and what the run asks of each function;
//! - the document's own fields and its unit's, but its functions;
//! - the document's tables: its definitions but where each is written, its types, and what its
//!   files table says of each file but its text;
//! - the function as it serializes, offsets included, so a function that moved derives again;
//! - for each file its spans lie in, its text from the comment lines core searches above the first
//!   span to the end of the last, with the line that text starts on.
//!
//! The tables count whole, which costs a function little where documents are small, as a Rust
//! build's batches are, and saves reading which entries each function reaches.
//!
//! A function whose identity another function of its document shares, or whose facts fail, is
//! derived every time, since what it derives depends on more than itself.
//!
//! Each document's records are kept as one facts pack: their texts in order, compressed together as
//! one zstd frame, so what functions repeat of each other compresses away, and an index of where
//! each text lies in it and its key, none for a function derived every time, named by the sha256
//! of its bytes like any object. A map entry per unit names the packs its documents made in the last run that
//! kept it, and a run looks its keys up in those.
//!
//! Each run also keeps a facts manifest, an object naming every map entry it read or wrote and
//! every pack those name, so a store elsewhere can take this run's facts and nothing older.
//!
//! A document read from stored bytes is also kept whole: an entry under the sha256 of those bytes
//! names its unit, its language and its pack. A run that reads the same bytes again takes every
//! record from the pack, in order, without decoding or parsing the document. Its functions' keys
//! would all be the same, being read from the document alone, and a function derived every time
//! derives the same record from the same document.

use super::super::model::{Definition, Document, File, Function, Unit, spans_reached};
use super::super::records::{self, Demand, Measured, Record};
use super::{Objects, sha256};
use crate::wording::Wording;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// The schema of a facts pack's index, and of each function's key.
pub const FACTS_SCHEMA: &str = "rstyle.function-facts/0";
/// The schema of a run's facts manifest.
pub const FACTS_MANIFEST_SCHEMA: &str = "rstyle.facts-manifest/0";
const MAGIC: &[u8; 6] = b"RSFACT";
const INDEX_MAGIC: &[u8; 8] = b"RSFACTIX";
/// The facts pack format's version, and the flags byte after it, which no version sets yet.
/// Version 1 compresses a pack's texts together; version 0 compressed each one alone.
const FORMAT: [u8; 2] = [1, 0];
const FOOTER: usize = 24;
/// The zstd level a pack's texts and its index are written at.
const LEVEL: i32 = 3;
/// The map from a unit, under one build and demand, to the facts packs its documents made.
const MAP: &str = "facts";
/// The map from a document's stored bytes, under one build and demand, to its facts pack.
const DOCUMENTS: &str = "facts-documents";
/// How many lines above a signature core searches for its comment, as `source` does.
const LOOKBACK_LINES: usize = super::super::source::LOOKBACK_LINES;

/// The function facts a local store keeps for runs of one rstyle build that ask one demand of
/// each function. Shared by a run's workers.
pub struct FunctionFacts {
    root: PathBuf,
    objects: Objects,
    /// The sha256 of the schema, the build and the demand, which every key starts from.
    scope: String,
    demand: Demand,
    verify: bool,
    /// Where each unit's kept frames lie, read once a run.
    units: Mutex<HashMap<String, Arc<OnceLock<Index>>>>,
    /// The packs each unit's documents made in this run.
    written: Mutex<BTreeMap<String, BTreeSet<String>>>,
    /// The entry of each document this run kept whole or read whole, by its key.
    documents: Mutex<BTreeMap<String, Value>>,
    reused: AtomicUsize,
    derived: AtomicUsize,
}

/// What a run of a store's function facts kept: how many functions it reused and derived, and the
/// object naming every entry and pack of its facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub reused: usize,
    pub derived: usize,
    pub manifest: String,
}

/// What a run of a store's function facts did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Functions whose records were read from the store.
    pub reused: usize,
    /// Functions whose records were derived, whether or not they could be kept.
    pub derived: usize,
}

/// Where the store keeps a unit's functions' frames: the packs its documents made in the last run
/// that kept it, and where in them each key's frame lies.
#[derive(Default)]
struct Index {
    packs: Vec<String>,
    frames: HashMap<String, (usize, [u64; 2])>,
}

/// A record read back from the store, and its text as kept.
type Found = Option<(Record, Vec<u8>)>;

/// The texts of a facts pack, in order, each with the key it's kept under, if any.
type Frames = Vec<(Option<String>, Vec<u8>)>;

/// Where each text of a facts pack lies in its texts, in order, with the key it's kept under, if
/// any.
type Places = Vec<(Option<String>, [u64; 2])>;

/// A document the store kept whole: its unit, and its records in document order.
pub struct KeptDocument {
    pub unit: String,
    pub records: Vec<Record>,
}

impl FunctionFacts {
    /// The facts the store at `root` keeps for runs of the rstyle build named `build` asking
    /// `demand` of each function. With `verify`, a run derives every function again and fails
    /// when a kept record differs from it.
    pub fn new(root: &Path, build: &str, demand: Demand, verify: bool) -> Self {
        let Demand {
            graphs,
            bounds,
            flow,
            defects,
        } = demand;
        let asked = format!("graphs={graphs} bounds={bounds} flow={flow} defects={defects}");
        FunctionFacts {
            root: root.to_path_buf(),
            objects: Objects::new(&root.join("objects")),
            scope: sha256(format!("{FACTS_SCHEMA}\n{build}\n{asked}").as_bytes()),
            demand,
            verify,
            units: Mutex::default(),
            written: Mutex::default(),
            documents: Mutex::default(),
            reused: AtomicUsize::new(0),
            derived: AtomicUsize::new(0),
        }
    }

    /// What the run asks of each function.
    pub fn demand(&self) -> Demand {
        self.demand
    }

    /// How many functions the run has reused and derived so far.
    pub fn counts(&self) -> Counts {
        Counts {
            reused: self.reused.load(Ordering::Relaxed),
            derived: self.derived.load(Ordering::Relaxed),
        }
    }

    /// A bundled document's records, in document order, as [`records::records`] reads them: each
    /// function's as kept under its key when the store holds it, else derived. The document's
    /// records are kept as a facts pack of their own, and kept whole under `digest`, the sha256
    /// its bytes were stored under, when there is one.
    pub fn records(
        &self,
        document: &Document,
        digest: Option<&str>,
    ) -> Result<Vec<Record>, String> {
        let unit = document.unit.as_ref().ok_or("The document has no unit")?;
        let functions = unit.functions.as_deref().unwrap_or_default();
        let keys = Keys::new(document, unit, &self.scope).each(functions);
        let index = self.index(&unit.id);
        let (found, spoiled) = self.found(&index, &keys, Wording::of(&unit.language.name));
        let wanted = |at: usize| self.verify || found[at].is_none();
        let (derived, failed) = records::read_some(document, self.demand, &wanted)?;
        if let Some(reason) = failed.into_values().next() {
            return Err(reason);
        }
        let places = (0..functions.len()).filter(|at| wanted(*at));
        let derived: BTreeMap<usize, Record> = places.zip(derived).collect();
        let (records, frames) = self.merged((keys, found), derived, digest.is_some())?;
        let pack = self.keep(&unit.id, frames, spoiled)?;
        if let Some(digest) = digest {
            self.keep_document(digest, unit, pack.as_deref())?;
        }
        Ok(records)
    }

    // rstyle-allow(side_effect_density): IO boundary: reads a document's entry and facts pack
    /// The records of the document whose bytes were stored under `digest`, when the store kept
    /// it whole and its pack reads, as deriving it again would give them. A run that verifies
    /// derives every document again instead.
    pub fn kept_document(&self, digest: &str) -> Option<KeptDocument> {
        if self.verify {
            return None;
        }
        let key = self.entry_key(digest);
        let entry = std::fs::read(self.root.join("maps").join(DOCUMENTS).join(&key)).ok()?;
        let entry: Value = serde_json::from_slice(&entry).ok()?;
        if entry["schema"] != FACTS_SCHEMA {
            return None;
        }
        let unit = entry["unit"].as_str()?.to_owned();
        let wording = Wording::of(entry["language"].as_str()?);
        let records = match &entry["pack"] {
            Value::Null => Vec::new(),
            Value::String(pack) => {
                let records = self.pack_records(pack, wording)?;
                let mut written = self.written.lock().expect("no panic while keeping facts");
                written
                    .entry(unit.clone())
                    .or_default()
                    .insert(pack.clone());
                records
            }
            _ => return None,
        };
        self.reused.fetch_add(records.len(), Ordering::Relaxed);
        self.noted(key, entry);
        Some(KeptDocument { unit, records })
    }

    // rstyle-allow(side_effect_density): IO boundary: reads a facts pack whole
    /// Every record a facts pack holds, in order, if each one reads.
    fn pack_records(&self, pack: &str, wording: Wording) -> Option<Vec<Record>> {
        let path = self.objects.path(pack).ok()?;
        let (entries, texts) = unpacked(&std::fs::read(path).ok()?).ok()?;
        entries
            .into_iter()
            .map(|(_, place)| decoded(text_at(&texts, place)?, wording).ok())
            .collect()
    }

    // rstyle-allow(side_effect_density): IO boundary: keeps a document's entry
    /// Keeps the document whose bytes were stored under `digest` whole: its unit, its language
    /// and the pack its records are in, none when it has no functions.
    fn keep_document(&self, digest: &str, unit: &Unit, pack: Option<&str>) -> Result<(), String> {
        let entry = json!({
            "schema": FACTS_SCHEMA,
            "unit": unit.id,
            "language": unit.language.name,
            "pack": pack,
        });
        let bytes = serde_json::to_vec(&entry).expect("a document entry serializes");
        super::replace(&self.document_path(digest), &bytes)?;
        self.noted(self.entry_key(digest), entry);
        Ok(())
    }

    /// Notes a document entry this run read or wrote, for its facts manifest.
    fn noted(&self, key: String, entry: Value) {
        let mut documents = self.documents.lock().expect("no panic while noting");
        documents.insert(key, entry);
    }

    fn document_path(&self, digest: &str) -> PathBuf {
        self.entry_path(DOCUMENTS, digest)
    }

    /// Each function's record in document order, kept or derived, and the text each one with a
    /// key is kept as, or each one at all when the document is kept `whole`. A record both kept
    /// and derived, as verifying gives, must be the same.
    fn merged(
        &self,
        (keys, found): (Vec<Option<String>>, Vec<Found>),
        mut derived: BTreeMap<usize, Record>,
        whole: bool,
    ) -> Result<(Vec<Record>, Frames), String> {
        let mut records = Vec::with_capacity(keys.len());
        let mut frames = Vec::new();
        for (at, (key, found)) in keys.into_iter().zip(found).enumerate() {
            let fresh = derived.remove(&at);
            let (record, frame) = match (found, key) {
                (Some((kept, text)), Some(key)) => {
                    if let Some(fresh) = &fresh {
                        same(&kept, fresh, &key)?;
                    }
                    self.reused.fetch_add(1, Ordering::Relaxed);
                    (kept, Some((Some(key), text)))
                }
                (_, key) => {
                    let fresh = fresh.ok_or("a function was neither kept nor derived")?;
                    let kept = key.is_some() || whole;
                    let frame = kept.then(|| Ok::<_, String>((key, encoded(&fresh)?)));
                    self.derived.fetch_add(1, Ordering::Relaxed);
                    (fresh, frame.transpose()?)
                }
            };
            records.push(record);
            frames.extend(frame);
        }
        Ok((records, frames))
    }

    /// Where the unit's kept frames lie, read the first time a run asks.
    fn index(&self, unit: &str) -> (String, Arc<OnceLock<Index>>) {
        let mut units = self.units.lock().expect("no panic while looking a unit up");
        let index = units.entry(unit.to_owned()).or_default();
        (unit.to_owned(), Arc::clone(index))
    }

    fn map_path(&self, unit: &str) -> PathBuf {
        self.entry_path(MAP, unit)
    }

    /// Where `map` keeps the entry for `name` under this store's scope.
    fn entry_path(&self, map: &str, name: &str) -> PathBuf {
        self.root.join("maps").join(map).join(self.entry_key(name))
    }

    /// The file name a map keeps `name`'s entry under in this store's scope.
    fn entry_key(&self, name: &str) -> String {
        sha256(format!("{}\n{name}", self.scope).as_bytes())
    }

    // rstyle-allow(side_effect_density): IO boundary: reads the indexes of a unit's facts packs
    /// Where the store keeps the unit's functions' frames. A pack it can't read is passed over, as
    /// if it held nothing: its functions are derived again.
    fn load(&self, unit: &str) -> Index {
        let mut index = Index::default();
        let Ok(entry) = std::fs::read(self.map_path(unit)) else {
            return index;
        };
        let entry: Value = serde_json::from_slice(&entry).unwrap_or_default();
        let packs = entry["packs"].as_array().into_iter().flatten();
        for name in packs.filter_map(Value::as_str) {
            let Ok(frames) = self.objects.path(name).and_then(|path| pack_index(&path)) else {
                continue;
            };
            let at = index.packs.len();
            let keyed = frames
                .into_iter()
                .filter_map(|(key, place)| Some((key?, (at, place))));
            index.frames.extend(keyed);
            index.packs.push(name.to_owned());
        }
        index
    }

    // rstyle-allow(side_effect_density): IO boundary: reads the packs a document's records are in
    /// The record kept under each key, with its frame, where the store holds one it can read, and
    /// whether any it held didn't read. Each pack they lie in is read once.
    fn found(
        &self,
        (unit, index): &(String, Arc<OnceLock<Index>>),
        keys: &[Option<String>],
        wording: Wording,
    ) -> (Vec<Found>, bool) {
        let index = index.get_or_init(|| self.load(unit));
        let mut by_pack: BTreeMap<usize, Vec<(usize, [u64; 2])>> = BTreeMap::new();
        for (at, key) in keys.iter().enumerate() {
            if let Some(&(pack, place)) = key.as_ref().and_then(|key| index.frames.get(key)) {
                by_pack.entry(pack).or_default().push((at, place));
            }
        }
        let mut found: Vec<Found> = (0..keys.len()).map(|_| None).collect();
        let mut spoiled = false;
        for (pack, places) in by_pack {
            let read_pack = |path: PathBuf| std::fs::read(path).map_err(|error| error.to_string());
            let bytes = self.objects.path(&index.packs[pack]).and_then(read_pack);
            let texts = bytes
                .and_then(|bytes| unpacked(&bytes))
                .map(|(_, texts)| texts);
            for (at, place) in places {
                let text = texts.as_ref().ok().and_then(|texts| text_at(texts, place));
                found[at] = text.and_then(|text| read(text, wording));
                spoiled |= found[at].is_none();
            }
        }
        (found, spoiled)
    }

    // rstyle-allow(side_effect_density): IO boundary: keeps a document's facts pack
    /// Keeps a document's frames as a facts pack of the unit's, unless it has none, and says
    /// which. When a pack the store held didn't read, a pack of the same name is written again,
    /// not trusted.
    fn keep(&self, unit: &str, frames: Frames, spoiled: bool) -> Result<Option<String>, String> {
        if frames.is_empty() {
            return Ok(None);
        }
        let bytes = pack(unit, frames)?;
        if spoiled {
            let path = self.objects.path(&sha256(&bytes))?;
            if path.exists() {
                std::fs::remove_file(&path).map_err(|error| error.to_string())?;
            }
        }
        let name = self.objects.put(&bytes)?;
        let mut written = self.written.lock().expect("no panic while keeping facts");
        written
            .entry(unit.to_owned())
            .or_default()
            .insert(name.clone());
        Ok(Some(name))
    }

    // rstyle-allow(side_effect_density): IO boundary: records each unit's facts packs
    /// Records the packs each unit's documents made in this run, so the next run looks its
    /// functions up in them, and keeps the run's facts manifest. Says how many functions the run
    /// reused and derived, and the manifest's name.
    pub fn finish(&self) -> Result<Kept, String> {
        let written = std::mem::take(&mut *self.written.lock().expect("no panic"));
        let mut units = serde_json::Map::new();
        for (unit, packs) in written {
            let entry = json!({"schema": FACTS_SCHEMA, "unit": unit, "packs": packs});
            let bytes = serde_json::to_vec(&entry).expect("a map entry serializes");
            super::replace(&self.map_path(&unit), &bytes)?;
            units.insert(self.entry_key(&unit), entry);
        }
        let documents = std::mem::take(&mut *self.documents.lock().expect("no panic"));
        let manifest = self.objects.put(&manifest(units, documents))?;
        let Counts { reused, derived } = self.counts();
        Ok(Kept {
            reused,
            derived,
            manifest,
        })
    }
}

/// A run's facts manifest: each map entry it read or wrote, by map and key, and each pack they
/// name, so a store that copies these holds this run's facts.
fn manifest(units: serde_json::Map<String, Value>, documents: BTreeMap<String, Value>) -> Vec<u8> {
    let packs = units
        .values()
        .flat_map(|entry| entry["packs"].as_array().into_iter().flatten());
    let pack = documents.values().map(|entry| &entry["pack"]);
    let objects: BTreeSet<&str> = packs.chain(pack).filter_map(Value::as_str).collect();
    let manifest = json!({
        "schema": FACTS_MANIFEST_SCHEMA,
        "maps": {MAP: units, DOCUMENTS: documents},
        "objects": objects,
    });
    serde_json::to_vec(&manifest).expect("a facts manifest serializes")
}

/// The record a text holds, with the text, if it reads: a record nested deeper than serde_json
/// reads doesn't, and is derived again.
fn read(text: &[u8], wording: Wording) -> Found {
    Some((decoded(text, wording).ok()?, text.to_vec()))
}

/// The text at `place` among a pack's texts.
fn text_at(texts: &[u8], [offset, length]: [u64; 2]) -> Option<&[u8]> {
    let offset = usize::try_from(offset).ok()?;
    let end = offset.checked_add(usize::try_from(length).ok()?)?;
    texts.get(offset..end)
}

/// A record from its kept text, in the unit's wording, which isn't kept.
pub(crate) fn decoded(text: &[u8], wording: Wording) -> Result<Record, String> {
    let mut record: Record = serde_json::from_slice(text).map_err(|error| error.to_string())?;
    record.wording = wording;
    if let Measured::Known(facts) | Measured::Body(facts) = &mut record.measured {
        facts.wording = wording;
    }
    Ok(record)
}

/// A record's text as a store keeps it, every field included.
pub(crate) fn encoded(record: &Record) -> Result<Vec<u8>, String> {
    crate::facts::keeping(|| serde_json::to_vec(record)).map_err(|error| error.to_string())
}

/// `text` compressed into a zstd frame that checks itself when it's read.
fn compressed(text: &[u8]) -> Result<Vec<u8>, String> {
    let checked = zstd::zstd_safe::CParameter::ChecksumFlag(true);
    let compress = || -> std::io::Result<Vec<u8>> {
        let mut compressor = zstd::bulk::Compressor::new(LEVEL)?;
        compressor.set_parameter(checked)?;
        compressor.compress(text)
    };
    compress().map_err(|error| error.to_string())
}

/// Refuses a kept record that differs from what deriving it again gives, saying where.
fn same(kept: &Record, fresh: &Record, key: &str) -> Result<(), String> {
    let (kept, derived) = (format!("{kept:?}"), format!("{fresh:?}"));
    if kept == derived {
        return Ok(());
    }
    let at = kept
        .bytes()
        .zip(derived.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let near =
        |text: &str| -> String { text.chars().skip(at.saturating_sub(60)).take(160).collect() };
    Err(format!(
        "{} ({}): the facts kept under {key} differ from what deriving it again gives\n  kept:    ...{}\n  derived: ...{}",
        fresh.name,
        fresh.identity,
        near(&kept),
        near(&derived)
    ))
}

/// A facts pack of `unit`'s functions: their texts in order, compressed together, then an index
/// saying where each text lies among them and which key it's kept under, if any, then where the
/// index lies.
fn pack(unit: &str, frames: Frames) -> Result<Vec<u8>, String> {
    let header = [MAGIC.as_slice(), &FORMAT].concat();
    let mut entries = Vec::with_capacity(frames.len());
    let mut at = 0u64;
    for (key, text) in &frames {
        entries.push(json!([key, at, text.len() as u64]));
        at += text.len() as u64;
    }
    let texts: Vec<&[u8]> = frames.iter().map(|(_, text)| text.as_slice()).collect();
    let texts = compressed(&texts.concat())?;
    let index = json!({"schema": FACTS_SCHEMA, "unit": unit, "functions": entries});
    let index = compressed(&serde_json::to_vec(&index).map_err(|error| error.to_string())?)?;
    let at = (header.len() + texts.len()) as u64;
    let length = (index.len() as u64).to_le_bytes();
    let footer = [&at.to_le_bytes()[..], &length, INDEX_MAGIC].concat();
    Ok([header, texts, index, footer].concat())
}

/// A facts pack's index and its texts, decompressed.
fn unpacked(bytes: &[u8]) -> Result<(Places, Vec<u8>), String> {
    let (entries, at) = index_of(&mut std::io::Cursor::new(bytes), "a facts pack")?;
    let end = usize::try_from(at).unwrap_or(usize::MAX);
    let texts = bytes
        .get(MAGIC.len() + FORMAT.len()..end)
        .unwrap_or_default();
    let texts = zstd::decode_all(texts).map_err(|error| error.to_string())?;
    Ok((entries, texts))
}

/// Where each text lies in the facts pack at `path`, in order, with its key if it has one, as its
/// index says, read without reading the texts.
fn pack_index(path: &Path) -> Result<Places, String> {
    let failed = |error: std::io::Error| format!("{}: {error}", path.display());
    let mut file = std::fs::File::open(path).map_err(failed)?;
    let (entries, _) = index_of(&mut file, &path.display().to_string())?;
    Ok(entries)
}

// rstyle-allow(side_effect_density): IO boundary: reads a facts pack's header, footer and index
/// A facts pack's index, read from its header, footer and index alone, and where its texts end.
fn index_of(pack: &mut (impl Read + Seek), named: &str) -> Result<(Places, u64), String> {
    let failed = |error: std::io::Error| format!("{named}: {error}");
    let header = read_at(pack, SeekFrom::Start(0), MAGIC.len() + FORMAT.len());
    if !header
        .map_err(failed)?
        .starts_with(&[MAGIC.as_slice(), &FORMAT[..1]].concat())
    {
        return Err(format!("{named} is not a facts pack this reads"));
    }
    let footer = read_at(pack, SeekFrom::End(-(FOOTER as i64)), FOOTER).map_err(failed)?;
    if &footer[16..] != INDEX_MAGIC {
        return Err(format!("{named} doesn't end with its index's place"));
    }
    let at = u64::from_le_bytes(footer[..8].try_into().expect("eight bytes"));
    let length = u64::from_le_bytes(footer[8..16].try_into().expect("eight bytes"));
    let length = usize::try_from(length).map_err(|error| error.to_string())?;
    let compressed = read_at(pack, SeekFrom::Start(at), length).map_err(failed)?;
    let text = zstd::decode_all(compressed.as_slice()).map_err(failed)?;
    Ok((index_entries(&text)?, at))
}

// rstyle-allow(side_effect_density): IO boundary: reads part of a file
/// The `length` bytes of `file` from `at`.
fn read_at(file: &mut (impl Read + Seek), at: SeekFrom, length: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = vec![0; length];
    file.seek(at)?;
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// The key, if any, and place of each text a facts pack's index lists.
fn index_entries(text: &[u8]) -> Result<Places, String> {
    let index: Value = serde_json::from_slice(text).map_err(|error| error.to_string())?;
    if index["schema"] != FACTS_SCHEMA {
        return Err("a facts pack of another schema".into());
    }
    let entries = index["functions"]
        .as_array()
        .ok_or("an index lists functions")?;
    entries
        .iter()
        .map(
            |entry| match (&entry[0], entry[1].as_u64(), entry[2].as_u64()) {
                (Value::String(key), Some(at), Some(length)) => {
                    Ok((Some(key.clone()), [at, length]))
                }
                (Value::Null, Some(at), Some(length)) => Ok((None, [at, length])),
                _ => Err("an index names a text by its key, offset and length".to_string()),
            },
        )
        .collect()
}

/// What a document's functions' keys share, worked out once for the document.
struct Keys<'a> {
    /// The sha256 of the scope, the document's own fields and its tables.
    shared: [u8; 32],
    definitions: HashMap<&'a str, &'a Definition>,
    files: HashMap<&'a str, &'a File>,
    /// How many of the document's functions each identity names.
    identities: HashMap<&'a str, usize>,
    /// Where each line of each file starts.
    lines: RefCell<HashMap<&'a str, Vec<usize>>>,
}

impl<'a> Keys<'a> {
    fn new(document: &'a Document, unit: &'a Unit, scope: &str) -> Self {
        let definitions: HashMap<&str, &Definition> = unit
            .definitions
            .iter()
            .map(|it| (it.id.as_str(), it))
            .collect();
        let functions = unit.functions.as_deref().unwrap_or_default();
        let mut identities: HashMap<&str, usize> = HashMap::new();
        for function in functions {
            if let Some(definition) = definitions.get(function.definition.as_str()) {
                *identities.entry(&definition.stable_identity).or_default() += 1;
            }
        }
        let mut shared = Hashing::default();
        shared.0.update(scope.as_bytes());
        shared.0.update(own_fields(document).as_bytes());
        let tables = (
            unit.definitions
                .iter()
                .map(Unspanned::of)
                .collect::<Vec<_>>(),
            &unit.types,
            unit.files.iter().map(Said::of).collect::<Vec<_>>(),
        );
        shared.write(&tables).expect("a unit's tables serialize");
        Keys {
            shared: shared.0.finalize().into(),
            definitions,
            files: unit.files.iter().map(|it| (it.id.as_str(), it)).collect(),
            identities,
            lines: RefCell::default(),
        }
    }

    /// Each function's key, in order: none for one whose identity another function shares.
    fn each(&self, functions: &[Function]) -> Vec<Option<String>> {
        functions
            .iter()
            .map(|function| self.key(function))
            .collect()
    }

    fn key(&self, function: &Function) -> Option<String> {
        let definition = self.definitions.get(function.definition.as_str())?;
        if self.identities.get(definition.stable_identity.as_str()) != Some(&1) {
            return None;
        }
        let mut hashing = Hashing::default();
        hashing.0.update(self.shared);
        let (written, reached) = spans_reached(|| hashing.write(function));
        written.ok()?;
        let mut hasher = hashing.0;
        hasher.update(b"\nfiles\n");
        for (id, &span) in &reached {
            let Some((id, file)) = self.files.get_key_value(id.as_str()) else {
                continue;
            };
            hasher.update(self.window(id, file.content.as_deref(), span).as_bytes());
        }
        Some(hex(&hasher.finalize()))
    }

    /// The text of a file from the comment lines searched above `first`'s line to `last`, as a
    /// digest, with where that text starts.
    fn window(&self, id: &'a str, content: Option<&str>, (first, last): (usize, usize)) -> String {
        let Some(content) = content else {
            return format!("{id} has no content\n");
        };
        let mut lines = self.lines.borrow_mut();
        let starts = lines.entry(id).or_insert_with(|| {
            std::iter::once(0)
                .chain(content.match_indices('\n').map(|(at, _)| at + 1))
                .collect()
        });
        let line = starts.partition_point(|&at| at <= first).max(1);
        let from_line = line.saturating_sub(1 + LOOKBACK_LINES);
        let from = starts[from_line].min(content.len());
        let to = last.clamp(from, content.len());
        let text = sha256(&content.as_bytes()[from..to]);
        format!("{id} line {from_line} bytes {from}..{to} {text}\n")
    }
}

/// A definition as a key reads it: all of it but where it's written, which nothing derived from a
/// function that names it reads.
#[derive(serde::Serialize)]
struct Unspanned<'a> {
    id: &'a str,
    kind: &'a super::super::model::DefinitionKind,
    name: &'a str,
    stable_identity: &'a str,
    scope: &'a super::super::model::Scope,
    owner: &'a Option<String>,
    modifiers: &'a [String],
    callable: &'a Option<super::super::model::Callable>,
    compilation: &'a Option<String>,
    standard: bool,
    destructor: bool,
    implements: &'a Option<String>,
    operation: &'a Option<String>,
    iteration: bool,
    extensions: &'a BTreeMap<String, Value>,
}

impl<'a> Unspanned<'a> {
    fn of(definition: &'a Definition) -> Self {
        let Definition {
            id,
            kind,
            name,
            stable_identity,
            scope,
            owner,
            modifiers,
            span: _,
            callable,
            compilation,
            standard,
            destructor,
            implements,
            operation,
            iteration,
            extensions,
        } = definition;
        Unspanned {
            id,
            kind,
            name,
            stable_identity,
            scope,
            owner,
            modifiers,
            callable,
            compilation,
            standard: *standard,
            destructor: *destructor,
            implements,
            operation,
            iteration: *iteration,
            extensions,
        }
    }
}

/// What the files table says of a file but its text, which a function's key reads only where
/// the function's spans lie.
#[derive(serde::Serialize)]
struct Said<'a> {
    id: &'a str,
    path: &'a str,
    origin: &'a str,
    generated_by: &'a Option<String>,
    test: bool,
}

impl<'a> Said<'a> {
    fn of(file: &'a File) -> Self {
        let File {
            id,
            path,
            digest: _,
            origin,
            generated_by,
            test,
            content: _,
        } = file;
        Said {
            id,
            path,
            origin,
            generated_by,
            test: *test,
        }
    }
}

/// A sha256 that JSON is written into, in large pieces.
#[derive(Default)]
struct Hashing(Sha256);

impl Hashing {
    fn write(&mut self, value: &impl serde::Serialize) -> Result<(), String> {
        let mut writer = std::io::BufWriter::with_capacity(1 << 16, &mut *self);
        serde_json::to_writer(&mut writer, value).map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())
    }
}

impl Write for Hashing {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What every function of a document reads of the document beyond its tables and functions:
/// its own fields and its unit's. Core reads none of the unit's namespaces, imports, values or
/// instances.
fn own_fields(document: &Document) -> String {
    let Document {
        schema_version,
        profile,
        layout,
        document: kind,
        producer,
        coverage,
        unit,
        unit_ref,
        function,
    } = document;
    let unit = unit.as_ref().map(|unit| {
        let Unit {
            id,
            root_file,
            language,
            toolchain,
            target,
            configuration,
            semantics,
            files: _,
            namespaces: _,
            imports: _,
            definitions: _,
            types: _,
            values: _,
            instances: _,
            functions: _,
            extensions,
        } = unit;
        json!({
            "id": id, "root_file": root_file, "language": language, "toolchain": toolchain,
            "target": target, "configuration": configuration, "semantics": semantics,
            "extensions": extensions,
        })
    });
    let own = json!({
        "schema_version": schema_version, "profile": profile, "layout": layout, "document": kind,
        "producer": producer, "coverage": coverage, "unit": unit, "unit_ref": unit_ref,
        "function": function.as_ref().map(|function| &function.id),
    });
    own.to_string()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(HEX[usize::from(byte >> 4)].into());
        text.push(HEX[usize::from(byte & 15)].into());
    }
    text
}

#[cfg(test)]
mod tests;
