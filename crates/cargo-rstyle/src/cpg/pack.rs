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
        .map(|unit| {
            let key = unit.key(level);
            if let Some(entry) = kept_entry(store, &objects, &key) {
                return Ok(entry);
            }
            let entry = pack_unit((capture, level), &objects, &sources, unit)?;
            keep_entry(store, &key, &entry)?;
            Ok(entry)
        })
        .collect::<Result<Vec<Value>, String>>()?;
    let total = |field: &str| -> u64 {
        entries
            .iter()
            .filter_map(|entry| entry[field].as_u64())
            .sum()
    };
    let run = json!({"schema": MANIFEST_SCHEMA, "units": entries});
    let manifest = objects.put(canonical(&run).as_bytes())?;
    Ok(
        json!({"manifest": manifest, "units": entries.len(), "functions": total("functions"), "pack_bytes": total("bytes")}),
    )
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
    UnitDocuments { id, positions, .. }: UnitDocuments,
) -> Result<Value, String> {
    let text = |position: usize| sources[position].text(capture);
    let mut documents = Vec::with_capacity(positions.len());
    for &position in &positions {
        documents.push(tables_only(&text(position)?)?);
    }
    let merged = store::merge_unit(documents)?;
    let mut packer = store::Packer::new(&merged, level)?;
    for framed in framed_in_parallel(&positions, &text, &merged, level)? {
        packer.add_framed(framed);
    }
    let packed = packer.finish()?;
    Ok(json!({
        "unit": id,
        "files": files(&merged),
        "pack": objects.put(&packed.bytes)?,
        "index": packed.index,
        "functions": packed.functions,
        "bytes": packed.bytes.len(),
    }))
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
