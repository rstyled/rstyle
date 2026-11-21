//! The run cache's local store. A unit's documents become one pack: the unit's tables once,
//! each function as a zstd frame of its own, and an index saying what each function is, what
//! it reads and where it lies, so a reader can tell what changed without decoding a frame.
//! Packs, and the manifest naming a run's packs, are kept as objects named by the sha256 of
//! their bytes.

use super::layout::canonical;
use super::source::comment_line_above;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

const PACK_MAGIC: &[u8; 6] = b"RSPACK";
const INDEX_MAGIC: &[u8; 8] = b"RSPACKIX";
/// The pack format's version, and the flags byte after it, which no version sets yet.
const FORMAT: [u8; 2] = [0, 0];
const FOOTER: usize = 24;
/// The schema of a run's manifest.
pub const MANIFEST_SCHEMA: &str = "rstyle.run-manifest/0";

/// A packed unit: its bytes, and where its index lies in them.
pub struct Packed {
    pub bytes: Vec<u8>,
    pub index: [u64; 2],
    pub functions: usize,
}

/// The lowercase hex sha256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest(value: &Value) -> String {
    sha256(&canonical_bytes(value))
}

/// Canonical JSON, as `layout::canonical` writes it. serde_json keeps an object's members
/// sorted by key, so its own output already is; a test holds the two to each other.
fn canonical_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("a JSON value serializes")
}

/// One bundled document of a unit from the bundled documents a frontend wrote it in. Every
/// table entry two documents both hold must be the same, since one of them is kept.
pub fn merge_unit(documents: Vec<Value>) -> Result<Value, String> {
    let mut documents = documents.into_iter();
    let mut merged = documents.next().ok_or("a unit has no documents")?;
    let mut seen: HashMap<(String, String), String> = HashMap::new();
    remember(&merged["unit"], &mut seen);
    for document in documents {
        let unit = &document["unit"];
        if unit["id"] != merged["unit"]["id"] {
            return Err("documents of different units can't be merged".into());
        }
        let functions = unit["functions"].as_array().cloned().unwrap_or_default();
        extend(&mut merged["unit"]["functions"], functions);
        for (table, entries) in tables(unit) {
            let fresh = unseen(unit, &table, entries, &mut seen)?;
            extend(&mut merged["unit"][table.as_str()], fresh);
        }
    }
    Ok(merged)
}

/// Adds `items` to the end of the array `target` is, starting one where the first document
/// had none of that table.
fn extend(target: &mut Value, items: Vec<Value>) {
    match target.as_array_mut() {
        Some(array) => array.extend(items),
        None => *target = Value::Array(items),
    }
}

/// What names a table entry: its ID, or the whole entry where it has none.
fn entry_key(entry: &Value) -> String {
    match entry["id"].as_str() {
        Some(id) => id.to_owned(),
        None => canonical(entry),
    }
}

fn remember(unit: &Value, seen: &mut HashMap<(String, String), String>) {
    for (table, entries) in tables(unit) {
        for entry in entries {
            seen.insert((table.clone(), entry_key(entry)), canonical(entry));
        }
    }
}

fn tables(unit: &Value) -> Vec<(String, &Vec<Value>)> {
    let Some(fields) = unit.as_object() else {
        return Vec::new();
    };
    fields
        .iter()
        .filter(|(name, _)| name.as_str() != "functions")
        .filter_map(|(name, value)| Some((name.clone(), value.as_array()?)))
        .collect()
}

/// The entries of one table that no earlier document held; an entry held with other
/// content is an error, since only one of the two could be kept.
fn unseen(
    unit: &Value,
    table: &str,
    entries: &[Value],
    seen: &mut HashMap<(String, String), String>,
) -> Result<Vec<Value>, String> {
    let mut fresh = Vec::new();
    for entry in entries {
        let key = (table.to_owned(), entry_key(entry));
        let text = canonical(entry);
        match seen.get(&key) {
            Some(kept) if *kept == text => {}
            Some(_) => {
                return Err(format!(
                    "{}: two documents disagree on {table} entry {}",
                    unit["id"].as_str().unwrap_or("a unit"),
                    key.1
                ));
            }
            None => {
                seen.insert(key, text);
                fresh.push(entry.clone());
            }
        }
    }
    Ok(fresh)
}

/// Packs a unit's bundled document at zstd `level`.
pub fn pack(document: &Value, level: i32) -> Result<Packed, String> {
    let mut packer = Packer::new(document, level)?;
    for function in document["unit"]["functions"]
        .as_array()
        .ok_or("a unit document lists its functions")?
    {
        packer.add(function)?;
    }
    packer.finish()
}

/// A pack written one function at a time, so a unit's functions never have to be held at
/// once: its tables come first, from a unit document whose functions are left out.
pub struct Packer<'a> {
    keys: Keys<'a>,
    bytes: Vec<u8>,
    level: i32,
    unit: Value,
    entries: Vec<Value>,
}

impl<'a> Packer<'a> {
    /// Starts the pack of `document`'s unit; any functions it lists are left for `add`.
    pub fn new(document: &'a Value, level: i32) -> Result<Self, String> {
        let head = without_functions(document)?;
        let mut bytes = PACK_MAGIC.to_vec();
        bytes.extend(FORMAT);
        let frame = append(&mut bytes, &canonical_bytes(&head), level)?;
        let unit = json!({"id": document["unit"]["id"], "digest": digest(&head), "frame": frame});
        Ok(Packer {
            keys: Keys::new(&document["unit"]),
            bytes,
            level,
            unit,
            entries: Vec::new(),
        })
    }

    /// Adds one of the unit's functions as a frame of its own.
    pub fn add(&mut self, function: &Value) -> Result<(), String> {
        let frame = append(&mut self.bytes, &canonical_bytes(function), self.level)?;
        self.entries.push(self.keys.entry(function, frame));
        Ok(())
    }

    /// The pack, with its index and footer.
    pub fn finish(mut self) -> Result<Packed, String> {
        let functions = self.entries.len();
        let index = json!({"unit": self.unit, "functions": self.entries});
        let written = append(&mut self.bytes, &canonical_bytes(&index), self.level)?;
        self.bytes.extend(footer(written));
        Ok(Packed {
            bytes: self.bytes,
            index: written,
            functions,
        })
    }
}

/// Where the index lies, and the mark that ends every pack.
fn footer([at, length]: [u64; 2]) -> Vec<u8> {
    [&at.to_le_bytes()[..], &length.to_le_bytes(), INDEX_MAGIC].concat()
}

/// The document with its unit's functions left out, without copying them.
fn without_functions(document: &Value) -> Result<Value, String> {
    let fields = document.as_object().ok_or("a document is an object")?;
    let unit = document["unit"]
        .as_object()
        .ok_or("a document holds its unit")?;
    let mut head = serde_json::Map::new();
    for (key, value) in fields.iter().filter(|(key, _)| key.as_str() != "unit") {
        head.insert(key.clone(), value.clone());
    }
    let mut kept = serde_json::Map::new();
    for (key, value) in unit.iter().filter(|(key, _)| key.as_str() != "functions") {
        kept.insert(key.clone(), value.clone());
    }
    kept.insert("functions".into(), json!([]));
    head.insert("unit".into(), Value::Object(kept));
    Ok(Value::Object(head))
}

/// Compresses `text` onto the end of `bytes`, and says where it went.
fn append(bytes: &mut Vec<u8>, text: &[u8], level: i32) -> Result<[u64; 2], String> {
    let frame = zstd::bulk::compress(text, level).map_err(|error| error.to_string())?;
    let at = bytes.len() as u64;
    bytes.extend(&frame);
    Ok([at, frame.len() as u64])
}

/// A pack's index.
pub fn index(pack: &[u8]) -> Result<Value, String> {
    if pack.len() < PACK_MAGIC.len() + FORMAT.len() + FOOTER || !pack.starts_with(PACK_MAGIC) {
        return Err("not a pack".into());
    }
    if pack[PACK_MAGIC.len()] != FORMAT[0] {
        return Err(format!("pack format {} is not one this reads", pack[6]));
    }
    let footer = &pack[pack.len() - FOOTER..];
    if &footer[16..] != INDEX_MAGIC {
        return Err("a pack ends with its index's place".into());
    }
    let at = u64::from_le_bytes(footer[..8].try_into().expect("eight bytes"));
    let length = u64::from_le_bytes(footer[8..16].try_into().expect("eight bytes"));
    serde_json::from_str(&frame(pack, [at, length])?).map_err(|error| error.to_string())
}

/// The text of the frame at `place` in `pack`.
pub fn frame(pack: &[u8], place: [u64; 2]) -> Result<String, String> {
    let [at, length] = place.map(|value| value as usize);
    let compressed = pack
        .get(at..at.saturating_add(length))
        .ok_or("a frame lies outside its pack")?;
    let text = zstd::decode_all(compressed).map_err(|error| error.to_string())?;
    String::from_utf8(text).map_err(|error| error.to_string())
}

fn place(value: &Value) -> Result<[u64; 2], String> {
    match (value[0].as_u64(), value[1].as_u64()) {
        (Some(at), Some(length)) => Ok([at, length]),
        _ => Err("an index names a frame by its offset and length".into()),
    }
}

/// The unit's bundled document, as it was packed.
pub fn unpack(pack: &[u8]) -> Result<Value, String> {
    let index = index(pack)?;
    let parse = |text: String| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string());
    let mut document = parse(frame(pack, place(&index["unit"]["frame"])?)?)?;
    let entries = index["functions"]
        .as_array()
        .ok_or("an index lists functions")?;
    let mut functions = Vec::with_capacity(entries.len());
    for entry in entries {
        functions.push(parse(frame(pack, place(&entry["frame"])?)?)?);
    }
    document["unit"]["functions"] = Value::Array(functions);
    Ok(document)
}

// rstyle-allow(side_effect_density): IO boundary: writes a document to its output
/// Writes the unit's bundled document, as it was packed, one function at a time: each
/// frame is already canonical JSON, so no unit's functions are ever held at once.
pub fn write_unpacked(pack: &[u8], out: &mut dyn std::io::Write) -> std::io::Result<()> {
    let invalid = |error: String| std::io::Error::other(error);
    let index = index(pack).map_err(invalid)?;
    let text = frame(pack, place(&index["unit"]["frame"]).map_err(invalid)?).map_err(invalid)?;
    let head: Value = serde_json::from_str(&text).map_err(std::io::Error::other)?;
    let entries = index["functions"].as_array().cloned().unwrap_or_default();
    write_object(&head, out, &mut |key, value, out| match key {
        "unit" => write_object(value, out, &mut |key, value, out| match key {
            "functions" => {
                out.write_all(b"[")?;
                for (position, entry) in entries.iter().enumerate() {
                    if position > 0 {
                        out.write_all(b",")?;
                    }
                    let place = place(&entry["frame"]).map_err(invalid)?;
                    out.write_all(frame(pack, place).map_err(invalid)?.as_bytes())?;
                }
                out.write_all(b"]")
            }
            _ => out.write_all(canonical(value).as_bytes()),
        }),
        _ => out.write_all(canonical(value).as_bytes()),
    })
}

type Member<'f> = dyn FnMut(&str, &Value, &mut dyn std::io::Write) -> std::io::Result<()> + 'f;

// rstyle-allow(side_effect_density, mutation_per_target): IO boundary: writes JSON to its output
/// An object's members in canonical order, each written by `member`.
fn write_object(
    object: &Value,
    out: &mut dyn std::io::Write,
    member: &mut Member<'_>,
) -> std::io::Result<()> {
    let mut fields: Vec<(&String, &Value)> = object.as_object().into_iter().flatten().collect();
    fields.sort_by(|a, b| a.0.cmp(b.0));
    out.write_all(b"{")?;
    for (position, (key, value)) in fields.into_iter().enumerate() {
        if position > 0 {
            out.write_all(b",")?;
        }
        out.write_all(Value::String(key.clone()).to_string().as_bytes())?;
        out.write_all(b":")?;
        member(key, value, out)?;
    }
    out.write_all(b"}")
}

/// What a function's index entry says of it, from its unit's tables and files.
struct Keys<'a> {
    /// Definitions and types by ID.
    entries: HashMap<&'a str, &'a Value>,
    types: BTreeSet<&'a str>,
    files: HashMap<&'a str, &'a Value>,
    /// Each type entry's references to other entries, worked out once for the unit.
    edges: std::cell::RefCell<HashMap<&'a str, Vec<&'a str>>>,
    /// Each entry's digest without spans, worked out once for the unit.
    digests: std::cell::RefCell<HashMap<&'a str, String>>,
}

impl<'a> Keys<'a> {
    fn new(unit: &'a Value) -> Self {
        let listed = |table: &str| -> Vec<&'a Value> {
            unit[table]
                .as_array()
                .map(|entries| entries.iter().collect())
                .unwrap_or_default()
        };
        let by_id = |entries: Vec<&'a Value>| -> HashMap<&'a str, &'a Value> {
            entries
                .into_iter()
                .filter_map(|entry| Some((entry["id"].as_str()?, entry)))
                .collect()
        };
        let types = by_id(listed("types"));
        let mut entries = by_id(listed("definitions"));
        entries.extend(types.iter().map(|(id, entry)| (*id, *entry)));
        Keys {
            entries,
            types: types.keys().copied().collect(),
            files: by_id(listed("files")),
            edges: Default::default(),
            digests: Default::default(),
        }
    }

    fn entry(&self, function: &Value, frame: [u64; 2]) -> Value {
        let span = &function["span"];
        let file = span["file"].as_str().unwrap_or_default();
        let start = span["start"].as_u64().unwrap_or_default();
        json!({
            "locator": function["id"],
            "frame": frame,
            "digest": digest(function),
            "shape": digest(&rebased(function, file, start)),
            "reads": self.reads(function),
            "context": self.context(function),
            "file": span["file"],
            "span": [span["start"], span["end"]],
        })
    }

    /// The definitions and types the function names, and every entry reachable from those
    /// through type entries, each without spans.
    fn reads(&self, function: &Value) -> String {
        let named = self.reached(function);
        let mut digests = self.digests.borrow_mut();
        let lines: Vec<String> = named
            .into_iter()
            .map(|id| {
                let entry = digests
                    .entry(id)
                    .or_insert_with(|| digest(&unspanned(self.entries[id])));
                format!("{id}={entry}\n")
            })
            .collect();
        sha256(lines.concat().as_bytes())
    }

    /// The entries `function` names, and every entry reachable from those through types.
    fn reached(&self, function: &Value) -> BTreeSet<&'a str> {
        let mut named = BTreeSet::new();
        self.referenced(function, &mut named);
        let mut todo: Vec<&'a str> = named.iter().copied().collect();
        while let Some(id) = todo.pop() {
            let edges = match self.types.contains(id) {
                true => self.edges(id),
                false => Vec::new(),
            };
            todo.extend(edges.into_iter().filter(|reached| named.insert(reached)));
        }
        named
    }

    /// The entries a type entry names.
    fn edges(&self, id: &'a str) -> Vec<&'a str> {
        if let Some(edges) = self.edges.borrow().get(id) {
            return edges.clone();
        }
        let mut named = BTreeSet::new();
        self.referenced(self.entries[id], &mut named);
        let edges: Vec<&'a str> = named.into_iter().collect();
        self.edges.borrow_mut().insert(id, edges.clone());
        edges
    }

    /// The table IDs a value names.
    fn referenced<'v>(&self, value: &'v Value, into: &mut BTreeSet<&'a str>) {
        match value {
            Value::String(text) => {
                if let Some((id, _)) = self.entries.get_key_value(text.as_str()) {
                    into.insert(*id);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| self.referenced(item, into)),
            Value::Object(fields) => fields.values().for_each(|item| self.referenced(item, into)),
            _ => {}
        }
    }

    /// What core reads from outside the function: the comment line above its signature,
    /// and what its file's entry says of the file.
    fn context(&self, function: &Value) -> String {
        let signature = match function["signature"]["span"].is_object() {
            true => &function["signature"]["span"],
            false => &function["span"],
        };
        let file = self
            .files
            .get(signature["file"].as_str().unwrap_or_default())
            .copied();
        let content = file
            .and_then(|file| file["content"].as_str())
            .unwrap_or_default();
        let start = signature["start"].as_u64().unwrap_or_default() as usize;
        let before = content.get(..start).unwrap_or(content);
        let line = 1 + before.matches('\n').count();
        let mut marks = file.cloned().unwrap_or(Value::Null);
        if let Some(fields) = marks.as_object_mut() {
            fields.remove("content");
            fields.remove("digest");
        }
        digest(&json!({"preceding": comment_line_above(content, line), "file": marks}))
    }
}

fn is_span(fields: &serde_json::Map<String, Value>) -> bool {
    fields.len() <= 4
        && ["file", "start", "end"]
            .iter()
            .all(|key| fields.contains_key(*key))
}

/// The value with every span left out.
fn unspanned(value: &Value) -> Value {
    match value {
        Value::Object(fields) if is_span(fields) => Value::Null,
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| (key.clone(), unspanned(item)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
        Value::Array(items) => items.iter().map(unspanned).collect(),
        scalar => scalar.clone(),
    }
}

/// The value with each span in `file` made relative to `base`.
fn rebased(value: &Value, file: &str, base: u64) -> Value {
    match value {
        Value::Object(fields) if is_span(fields) && fields["file"] == file => {
            let mut span = fields.clone();
            for key in ["start", "end"] {
                if let Some(offset) = span[key].as_u64() {
                    span[key] = json!(offset as i64 - base as i64);
                }
            }
            span.into()
        }
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| (key.clone(), rebased(item, file, base)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
        Value::Array(items) => items.iter().map(|item| rebased(item, file, base)).collect(),
        scalar => scalar.clone(),
    }
}

// rstyle-allow(side_effect_density): IO boundary: reads a document file
/// The text of a document file written without a manifest: zstd-compressed when its name
/// ends in `.zst`, plain otherwise.
pub fn document_text(path: &Path) -> Result<String, String> {
    let stored = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = match path.extension().is_some_and(|extension| extension == "zst") {
        true => zstd::decode_all(stored.as_slice()).map_err(|error| error.to_string())?,
        false => stored,
    };
    String::from_utf8(text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Objects named by the sha256 of their bytes, in a directory.
pub struct Objects {
    root: PathBuf,
}

impl Objects {
    pub fn new(root: &Path) -> Self {
        Objects {
            root: root.to_path_buf(),
        }
    }

    fn path(&self, name: &str) -> Result<PathBuf, String> {
        let valid = name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !valid {
            return Err(format!("{name} is not an object's name"));
        }
        Ok(self.root.join(&name[..2]).join(&name[2..]))
    }

    // rstyle-allow(side_effect_density): IO boundary: writes an object once, through a rename
    /// Keeps `bytes`, and names them. An object already kept is left as it is.
    pub fn put(&self, bytes: &[u8]) -> Result<String, String> {
        let name = sha256(bytes);
        let path = self.path(&name)?;
        if path.exists() {
            return Ok(name);
        }
        let directory = path.parent().expect("an object has a directory");
        std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let staged = directory.join(format!(".{name}.{}", std::process::id()));
        std::fs::write(&staged, bytes).map_err(|error| error.to_string())?;
        std::fs::rename(&staged, &path).map_err(|error| error.to_string())?;
        Ok(name)
    }

    // rstyle-allow(side_effect_density): IO boundary: reads an object and checks its name
    /// The object named `name`, refused unless its bytes still hash to the name.
    pub fn get(&self, name: &str) -> Result<Vec<u8>, String> {
        let bytes = std::fs::read(self.path(name)?).map_err(|error| format!("{name}: {error}"))?;
        match sha256(&bytes) == name {
            true => Ok(bytes),
            false => Err(format!("{name}: its bytes no longer match its name")),
        }
    }
}

#[cfg(test)]
mod tests;
