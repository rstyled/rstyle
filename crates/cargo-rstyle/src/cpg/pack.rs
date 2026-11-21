//! `rstyle cpg pack` and `rstyle cpg unpack`: a run's documents into a local store as one
//! pack per unit, named by a manifest, and back out as documents any command reads.

use crate::{CpgPackArgs, CpgUnpackArgs};
use rstyle_core::cpg::layout::canonical;
use rstyle_core::cpg::store::{self, MANIFEST_SCHEMA, Objects};
use rstyle_core::cpg::transport::{
    self, DOCUMENTS_MANIFEST, DocumentManifest, Encoding, SealedDocument,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(crate) fn pack(args: &CpgPackArgs) -> Result<(String, u8), String> {
    let summary = pack_run(&args.capture, &args.store, args.level)?;
    Ok((format!("{summary}\n"), 0))
}

// rstyle-allow(side_effect_density): IO boundary: reads a run and writes its packs
/// Keeps the run whose documents are in `capture` in `store`, one pack per unit at zstd
/// `level`, and says what it kept: the manifest naming the run, and its size.
pub(crate) fn pack_run(capture: &Path, store: &Path, level: i32) -> Result<Value, String> {
    let objects = Objects::new(&store.join("objects"));
    let sources = sources(capture)?;
    let entries = units(&sources, capture)?
        .into_iter()
        .map(|unit| reused_or_packed((capture, store, level), &objects, &sources, &unit))
        .collect::<Result<Vec<Value>, String>>()?;
    manifest(&objects, entries)
}

/// A unit's entry in the run's manifest: as it was kept the last time its documents were
/// packed, else packed now and recorded under its documents.
fn reused_or_packed(
    (capture, store, level): (&Path, &Path, i32),
    objects: &Objects,
    sources: &[Source],
    unit: &UnitDocuments,
) -> Result<Value, String> {
    let key = unit.key(level);
    if let Some(entry) = kept_entry(store, objects, &key) {
        return Ok(entry);
    }
    let entry = pack_unit((capture, level), objects, sources, &unit.positions)?;
    keep_entry(store, &key, &entry)?;
    Ok(entry)
}

/// One unit's documents, by position, and what keys its pack: the sha256 of each of its
/// documents' text, in order.
struct UnitDocuments {
    id: String,
    positions: Vec<usize>,
    digests: Vec<String>,
}

impl UnitDocuments {
    /// The key a unit's pack is kept under: its documents exactly, and the level they were
    /// packed at.
    fn key(&self, level: i32) -> String {
        let text = format!("{MANIFEST_SCHEMA}\n{level}\n{}", self.digests.join("\n"));
        store::sha256(text.as_bytes())
    }
}

/// Each unit's documents, in the order units first appear. Only each document's unit is read
/// here, so a run never has to fit in memory at once.
fn units(sources: &[Source], capture: &Path) -> Result<Vec<UnitDocuments>, String> {
    let mut units: Vec<UnitDocuments> = Vec::new();
    for (position, source) in sources.iter().enumerate() {
        let text = source.text(capture)?;
        let named: Named =
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", source.name()))?;
        let digest = store::sha256(text.as_bytes());
        match units.iter_mut().find(|unit| unit.id == named.unit.id) {
            Some(unit) => {
                unit.positions.push(position);
                unit.digests.push(digest);
            }
            None => units.push(UnitDocuments {
                id: named.unit.id,
                positions: vec![position],
                digests: vec![digest],
            }),
        }
    }
    Ok(units)
}

/// A unit's entry in the run's manifest, as it was kept the last time its documents were
/// packed, if its pack is still in the store.
fn kept_entry(store: &Path, objects: &Objects, key: &str) -> Option<Value> {
    let entry: Value = serde_json::from_slice(&std::fs::read(map_path(store, key)).ok()?).ok()?;
    let pack = entry["pack"].as_str()?;
    objects.has(pack).then_some(entry)
}

fn map_path(store: &Path, key: &str) -> PathBuf {
    store.join("maps").join("units").join(key)
}

// rstyle-allow(side_effect_density): IO boundary: records which pack a unit's documents made
/// Records that the documents `key` names made the unit `entry`, so a later run of the same
/// documents reuses it.
fn keep_entry(store: &Path, key: &str, entry: &Value) -> Result<(), String> {
    let path = map_path(store, key);
    let directory = path.parent().expect("a map entry has a directory");
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let staged = directory.join(format!(".{key}.{}", std::process::id()));
    std::fs::write(&staged, canonical(entry)).map_err(|error| error.to_string())?;
    std::fs::rename(&staged, &path).map_err(|error| error.to_string())
}

/// Packs one unit and keeps its pack: its tables first, with every function left out, then
/// each document again, one function at a time, so no more than one document's functions
/// is ever held. Returns the unit's entry in the run's manifest.
fn pack_unit(
    (capture, level): (&Path, i32),
    objects: &Objects,
    sources: &[Source],
    positions: &[usize],
) -> Result<Value, String> {
    let text = |position: usize| sources[position].text(capture);
    let merged = store::merge_documents(
        positions
            .iter()
            .map(|&position| tables_only(&text(position)?)),
    )?;
    let mut packer = store::Packer::new(&merged, level)?;
    for framed in framed_in_parallel(positions, &text, &merged, level)? {
        packer.add_framed(framed);
    }
    let packed = packer.finish()?;
    let id = merged["unit"]["id"]
        .as_str()
        .ok_or("a document names its unit")?;
    kept(objects, id.to_owned(), &merged, packed)
}

/// A unit's entry in the run's manifest, once its pack is kept.
fn kept(
    objects: &Objects,
    id: String,
    merged: &Value,
    packed: store::Packed,
) -> Result<Value, String> {
    Ok(json!({
        "unit": id,
        "files": files(merged),
        "pack": objects.put(&packed.bytes)?,
        "index": packed.index,
        "functions": packed.functions,
        "bytes": packed.bytes.len(),
    }))
}

/// What a run knows of its units before it's kept. A document holding the whole of its unit,
/// from a frontend's stream, is packed as it arrives; a unit whose documents' seals name their
/// text, as Rust's capture writes them, is named by them, so only the units that changed are
/// read again. Either way keeping the run reads no more than it has to.
pub(crate) struct Emitted {
    store: PathBuf,
    level: i32,
    units: Mutex<Vec<(usize, String, Value)>>,
    /// Units whose documents' seals say what text they hold: for each, the sha256 of its
    /// documents' text, in the run's order.
    sealed: Mutex<Vec<Vec<String>>>,
}

impl Emitted {
    pub(crate) fn new(store: &Path, level: i32) -> Self {
        Emitted {
            store: store.to_path_buf(),
            level,
            units: Default::default(),
            sealed: Default::default(),
        }
    }

    // rstyle-allow(side_effect_density): IO boundary: packs a document as it arrives
    /// Packs the stream's document `index`, whose `text` is a unit's bundled document, unless
    /// the store already holds a pack of exactly this document.
    pub(crate) fn document(&self, index: usize, text: &str) -> Result<(), String> {
        let objects = Objects::new(&self.store.join("objects"));
        let document = parse(text)?;
        let id = document["unit"]["id"]
            .as_str()
            .ok_or("a document names its unit")?
            .to_owned();
        let digests = vec![store::sha256(text.as_bytes())];
        let unit = UnitDocuments {
            id: id.clone(),
            positions: vec![index],
            digests,
        };
        let key = unit.key(self.level);
        let entry = match kept_entry(&self.store, &objects, &key) {
            Some(entry) => entry,
            None => {
                let packed = store::pack(&document, self.level)?;
                let entry = kept(&objects, id.clone(), &document, packed)?;
                keep_entry(&self.store, &key, &entry)?;
                entry
            }
        };
        let mut units = self.units.lock().expect("no panic while packing");
        units.push((index, id, entry));
        Ok(())
    }

    /// Notes the run's next unit by the sha256 of its documents' text, as their seals say,
    /// so keeping the run reads only the units no earlier run kept.
    pub(crate) fn sealed_unit(&self, digests: Vec<String>) {
        let mut sealed = self.sealed.lock().expect("no panic while packing");
        sealed.push(digests);
    }

    /// The store the run is kept in.
    pub(crate) fn store(&self) -> &Path {
        &self.store
    }

    /// The run of `documents` saved in `capture`, kept: from its units' seals when they name
    /// every document, else from what was packed as it arrived when each came as its own unit,
    /// else packed from the saved documents, as a run whose documents didn't stream, or whose
    /// unit came in several, is.
    pub(crate) fn finish(self, capture: &Path, documents: usize) -> Result<Value, String> {
        let sealed = self.sealed.into_inner().expect("no panic while packing");
        if !sealed.is_empty() {
            return match sealed.iter().map(Vec::len).sum::<usize>() == documents {
                true => sealed_run((capture, &self.store, self.level), sealed),
                false => pack_run(capture, &self.store, self.level),
            };
        }
        let mut units = self.units.into_inner().expect("no panic while packing");
        units.sort_by_key(|(index, ..)| *index);
        let ids: std::collections::BTreeSet<&String> = units.iter().map(|(_, id, _)| id).collect();
        if ids.len() != units.len() || units.len() != documents {
            return pack_run(capture, &self.store, self.level);
        }
        let entries: Vec<Value> = units.into_iter().map(|(_, _, entry)| entry).collect();
        manifest(&Objects::new(&self.store.join("objects")), entries)
    }
}

// rstyle-allow(side_effect_density): IO boundary: reads a run's changed units and writes their packs
/// Keeps the run saved in `capture` whose units' documents, in order, hold the text `sealed`
/// names: a unit kept before is named, not read. Units that turn out to share an ID are
/// packed as one, as [`pack_run`] does.
fn sealed_run(
    (capture, store, level): (&Path, &Path, i32),
    sealed: Vec<Vec<String>>,
) -> Result<Value, String> {
    let objects = Objects::new(&store.join("objects"));
    let sources = sources(capture)?;
    let mut next = 0;
    let mut entries = Vec::with_capacity(sealed.len());
    for digests in sealed {
        let positions: Vec<usize> = (next..next + digests.len()).collect();
        next += digests.len();
        let unit = UnitDocuments {
            id: String::new(),
            positions,
            digests,
        };
        entries.push(reused_or_packed(
            (capture, store, level),
            &objects,
            &sources,
            &unit,
        )?);
    }
    let ids: std::collections::BTreeSet<&str> = entries
        .iter()
        .filter_map(|entry| entry["unit"].as_str())
        .collect();
    if ids.len() != entries.len() {
        return pack_run(capture, store, level);
    }
    manifest(&objects, entries)
}

/// Names the run whose units are `entries`, and says what it kept.
fn manifest(objects: &Objects, entries: Vec<Value>) -> Result<Value, String> {
    let total = |field: &str| -> u64 {
        entries
            .iter()
            .filter_map(|entry| entry[field].as_u64())
            .sum()
    };
    let (units, functions, bytes) = (entries.len(), total("functions"), total("bytes"));
    let run = json!({"schema": MANIFEST_SCHEMA, "units": entries});
    let manifest = objects.put(canonical(&run).as_bytes())?;
    Ok(json!({"manifest": manifest, "units": units, "functions": functions, "pack_bytes": bytes}))
}

/// The unit's functions framed, in document order, its documents shared among threads. Each
/// thread holds one document at a time and the frames it has compressed.
fn framed_in_parallel(
    positions: &[usize],
    text: &(dyn Fn(usize) -> Result<String, String> + Sync),
    merged: &Value,
    level: i32,
) -> Result<Vec<store::Framed>, String> {
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get());
    let share = positions.len().div_ceil(threads).max(1);
    let framed = std::thread::scope(|scope| {
        let workers: Vec<_> = positions
            .chunks(share)
            .map(|chunk| scope.spawn(move || frame_documents(chunk, text, merged, level)))
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("a packing thread doesn't panic"))
            .collect::<Result<Vec<_>, String>>()
    })?;
    Ok(framed.into_iter().flatten().collect())
}

/// Each function of the documents at `positions`, framed against the unit's tables.
fn frame_documents(
    positions: &[usize],
    text: &(dyn Fn(usize) -> Result<String, String> + Sync),
    merged: &Value,
    level: i32,
) -> Result<Vec<store::Framed>, String> {
    let framer = store::Framer::new(merged, level);
    let mut framed = Vec::new();
    for &position in positions {
        let document = parse(&text(position)?)?;
        for function in document["unit"]["functions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            framed.push(framer.frame(function)?);
        }
    }
    Ok(framed)
}

/// A document's unit, read without the rest of it.
#[derive(Deserialize)]
struct Named {
    unit: NamedUnit,
}

#[derive(Deserialize)]
struct NamedUnit {
    id: String,
}

fn parse(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|error| error.to_string())
}

/// A document without its unit's functions, which are skipped as they are read rather
/// than built and dropped: most of a document is its functions.
#[derive(Deserialize)]
struct TablesOnly {
    unit: UnitTables,
    #[serde(flatten)]
    head: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct UnitTables {
    #[serde(rename = "functions")]
    _functions: serde::de::IgnoredAny,
    #[serde(flatten)]
    tables: serde_json::Map<String, Value>,
}

fn tables_only(text: &str) -> Result<Value, String> {
    let document: TablesOnly = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let mut unit = document.unit.tables;
    unit.insert("functions".into(), json!([]));
    let mut head = document.head;
    head.insert("unit".into(), Value::Object(unit));
    Ok(Value::Object(head))
}

/// The paths a unit's documents hold, sorted: how a diff's changed paths find their units.
fn files(document: &Value) -> Vec<&str> {
    let mut paths: Vec<&str> = document["unit"]["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|file| file["path"].as_str())
        .collect();
    paths.sort_unstable();
    paths
}

/// Where a run's document is: sealed in its manifest, or a file of the directory.
enum Source {
    Sealed(SealedDocument),
    File(PathBuf),
}

impl Source {
    fn text(&self, directory: &Path) -> Result<String, String> {
        match self {
            Source::Sealed(seal) => transport::read_sealed_text(directory, seal),
            Source::File(path) => store::document_text(path),
        }
    }

    fn name(&self) -> String {
        match self {
            Source::Sealed(seal) => seal.file.clone(),
            Source::File(path) => path.display().to_string(),
        }
    }
}

// rstyle-allow(side_effect_density): IO boundary: lists a run's documents
/// A run's documents: through its manifest where it has one, otherwise every document in
/// the directory in name order.
fn sources(directory: &Path) -> Result<Vec<Source>, String> {
    if directory.join(DOCUMENTS_MANIFEST).exists() {
        let manifest = DocumentManifest::read(&transport::read_manifest_bytes(directory)?)?;
        return Ok(manifest.documents.into_iter().map(Source::Sealed).collect());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| {
            let name = path.to_string_lossy();
            name.ends_with(".cpg.json") || name.ends_with(".cpg.json.zst")
        })
        .collect();
    paths.sort();
    Ok(paths.into_iter().map(Source::File).collect())
}

// rstyle-allow(side_effect_density): IO boundary: reads packs and writes their documents
pub(crate) fn unpack(args: &CpgUnpackArgs) -> Result<(String, u8), String> {
    let objects = Objects::new(&args.store.join("objects"));
    let run: Value =
        serde_json::from_slice(&objects.get(&args.manifest)?).map_err(|error| error.to_string())?;
    if run["schema"] != MANIFEST_SCHEMA {
        return Err(format!("{} is not a run manifest", args.manifest));
    }
    std::fs::create_dir_all(&args.out).map_err(|error| error.to_string())?;
    let units = run["units"]
        .as_array()
        .ok_or("a run manifest lists its units")?;
    let mut seals = Vec::with_capacity(units.len());
    for (position, unit) in units.iter().enumerate() {
        let name = unit["pack"].as_str().ok_or("a unit names its pack")?;
        let bytes = objects.get(name)?;
        let path = args
            .out
            .join(format!("document-{position:06}.cpg.json.zst"));
        seals.push(transport::publish(&path, Encoding::Zstd, |writer| {
            store::write_unpacked(&bytes, writer)
        })?);
    }
    let manifest = serde_json::to_vec(&DocumentManifest::of(seals)).map_err(|e| e.to_string())?;
    std::fs::write(args.out.join(DOCUMENTS_MANIFEST), manifest).map_err(|e| e.to_string())?;
    Ok((
        format!(
            "{} documents written to {}\n",
            units.len(),
            args.out.display()
        ),
        0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(functions: &[&str]) -> String {
        unit_document("unit:t", functions)
    }

    fn unit_document(unit: &str, functions: &[&str]) -> String {
        let functions: Vec<Value> = functions
            .iter()
            .map(|id| json!({"id": id, "span": {"file": "file:a.rs", "start": 0, "end": 1}}))
            .collect();
        let document = json!({
            "schema_version": "0.1.0", "profile": {"id": "compat.cpg", "version": "0.1.0"},
            "producer": {"name": "test", "version": "0"}, "coverage": {}, "layout": "bundled",
            "document": "unit",
            "unit": {"id": unit, "files": [], "definitions": [], "types": [], "functions": functions},
        });
        serde_json::to_string(&document).unwrap()
    }

    #[test]
    fn a_unit_in_one_document_is_kept_as_it_arrives_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        let text = document(&["fn:a"]);
        let first = Emitted::new(&store, 3);
        first.document(0, &text).unwrap();
        let first = first.finish(dir.path(), 1).unwrap();
        let second = Emitted::new(&store, 3);
        second.document(0, &text).unwrap();
        assert_eq!(second.store(), store);
        assert_eq!(second.finish(dir.path(), 1).unwrap(), first);
    }

    #[test]
    fn a_unit_in_several_documents_is_packed_from_the_saved_run() {
        let dir = tempfile::tempdir().unwrap();
        let emitted = Emitted::new(&dir.path().join("store"), 3);
        for (index, function) in ["fn:a", "fn:b"].iter().enumerate() {
            let text = document(&[function]);
            std::fs::write(dir.path().join(format!("document-{index}.cpg.json")), &text).unwrap();
            emitted.document(index, &text).unwrap();
        }
        let kept = emitted.finish(dir.path(), 2).unwrap();
        assert_eq!(
            (kept["units"].as_u64(), kept["functions"].as_u64()),
            (Some(1), Some(2))
        );
    }

    #[test]
    fn a_run_whose_documents_did_not_stream_is_packed_from_the_saved_run() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("document-0.cpg.json"), document(&["fn:a"])).unwrap();
        let kept = Emitted::new(&dir.path().join("store"), 3)
            .finish(dir.path(), 1)
            .unwrap();
        assert_eq!(kept["units"].as_u64(), Some(1));
    }

    /// A saved run of `documents`, each a unit's ID and its functions, and the sha256 of each
    /// document's text.
    fn saved_run(dir: &Path, documents: &[(&str, &[&str])]) -> Vec<String> {
        documents
            .iter()
            .enumerate()
            .map(|(index, (unit, functions))| {
                let text = unit_document(unit, functions);
                std::fs::write(dir.join(format!("document-{index}.cpg.json")), &text).unwrap();
                store::sha256(text.as_bytes())
            })
            .collect()
    }

    #[test]
    fn units_named_by_their_seals_are_kept_as_packing_the_run_keeps_them() {
        let dir = tempfile::tempdir().unwrap();
        let run = dir.path().join("run");
        std::fs::create_dir(&run).unwrap();
        let digests = saved_run(
            &run,
            &[
                ("unit:t", &["fn:a"]),
                ("unit:t", &["fn:b"]),
                ("unit:u", &["fn:c"]),
            ],
        );
        let sealed = Emitted::new(&dir.path().join("store"), 3);
        sealed.sealed_unit(digests[..2].to_vec());
        sealed.sealed_unit(digests[2..].to_vec());
        let kept = sealed.finish(&run, 3).unwrap();
        let packed = pack_run(&run, &dir.path().join("packed"), 3).unwrap();
        assert_eq!(kept, packed);

        // A unit kept before is named by its seals alone: its documents aren't read again.
        for index in 0..3 {
            std::fs::write(run.join(format!("document-{index}.cpg.json")), "not JSON").unwrap();
        }
        let again = Emitted::new(&dir.path().join("store"), 3);
        again.sealed_unit(digests[..2].to_vec());
        again.sealed_unit(digests[2..].to_vec());
        assert_eq!(again.finish(&run, 3).unwrap(), kept);
    }

    #[test]
    fn seals_that_miss_documents_or_split_a_unit_leave_the_run_to_be_packed() {
        let dir = tempfile::tempdir().unwrap();
        let digests = saved_run(dir.path(), &[("unit:t", &["fn:a"]), ("unit:t", &["fn:b"])]);
        let missing = Emitted::new(&dir.path().join("missing"), 3);
        missing.sealed_unit(digests[..1].to_vec());
        let split = Emitted::new(&dir.path().join("split"), 3);
        split.sealed_unit(digests[..1].to_vec());
        split.sealed_unit(digests[1..].to_vec());
        for emitted in [missing, split] {
            let kept = emitted.finish(dir.path(), 2).unwrap();
            assert_eq!(
                (kept["units"].as_u64(), kept["functions"].as_u64()),
                (Some(1), Some(2))
            );
        }
    }
}
