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

// rstyle-allow(side_effect_density): IO boundary: reads a run and writes its packs
pub(crate) fn pack(args: &CpgPackArgs) -> Result<(String, u8), String> {
    let objects = Objects::new(&args.store.join("objects"));
    let sources = sources(&args.capture)?;
    let entries = units(&sources, &args.capture)?
        .into_iter()
        .map(|unit| pack_unit(args, &objects, &sources, unit))
        .collect::<Result<Vec<Value>, String>>()?;
    let total = |field: &str| -> u64 {
        entries
            .iter()
            .filter_map(|entry| entry[field].as_u64())
            .sum()
    };
    let run = json!({"schema": MANIFEST_SCHEMA, "units": entries});
    let manifest = objects.put(canonical(&run).as_bytes())?;
    let summary = json!({"manifest": manifest, "units": entries.len(), "functions": total("functions"), "pack_bytes": total("bytes")});
    Ok((format!("{summary}\n"), 0))
}

/// Each unit's documents, by position, in the order units first appear. Only each
/// document's unit is read here, so a run never has to fit in memory at once.
fn units(sources: &[Source], capture: &Path) -> Result<Vec<(String, Vec<usize>)>, String> {
    let mut units: Vec<(String, Vec<usize>)> = Vec::new();
    for (position, source) in sources.iter().enumerate() {
        let named: Named = serde_json::from_str(&source.text(capture)?)
            .map_err(|error| format!("{}: {error}", source.name()))?;
        match units.iter_mut().find(|(unit, _)| *unit == named.unit.id) {
            Some((_, positions)) => positions.push(position),
            None => units.push((named.unit.id, vec![position])),
        }
    }
    Ok(units)
}

/// Packs one unit and keeps its pack: its tables first, with every function left out, then
/// each document again, one function at a time, so no more than one document's functions
/// is ever held. Returns the unit's entry in the run's manifest.
fn pack_unit(
    args: &CpgPackArgs,
    objects: &Objects,
    sources: &[Source],
    (id, positions): (String, Vec<usize>),
) -> Result<Value, String> {
    let text = |position: usize| sources[position].text(&args.capture);
    let mut documents = Vec::with_capacity(positions.len());
    for &position in &positions {
        let mut document = parse(&text(position)?)?;
        document["unit"]["functions"] = json!([]);
        documents.push(document);
    }
    let merged = store::merge_unit(documents)?;
    let mut packer = store::Packer::new(&merged, args.level)?;
    for &position in &positions {
        let document = parse(&text(position)?)?;
        for function in document["unit"]["functions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            packer.add(function)?;
        }
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
