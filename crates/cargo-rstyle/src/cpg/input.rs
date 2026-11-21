//! Files, captures, and pipes share incremental derivation and receipt handling.
use super::receipt::{Receipt, TransportProgress};
use crate::{CpgInputArgs, CpgReceiveArgs, Format};
use rstyle_core::cpg::{
    self, OnRecords, Order, Pool, keep_records,
    layout::{self, Parsed},
    records::{Demand, Record},
    transport::{
        self, CaptureReader, Captured, DocumentManifest, Encoding, SealedDocument, SequenceReader,
    },
};
use std::collections::{BTreeMap, btree_map::Entry};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// More workers than this stopped helping when measured on Go and Rust runs.
const MAX_WORKERS: usize = 8;
pub(crate) const STREAM_BUFFER: usize = 1 << 20;

pub(super) struct Received {
    pub records: Vec<Record>,
    pub receipt: Option<Receipt>,
}

pub(super) enum InputError {
    Documents(String),
    Rejected(Box<Receipt>),
}

impl InputError {
    pub(super) fn output(self, format: Format) -> Result<(String, u8), String> {
        match self {
            Self::Rejected(receipt) => receipt.output(format),
            Self::Documents(message) => Err(message),
        }
    }
}

/// The records of whichever input was selected, read and derived on a pool, in `order`.
pub(super) fn read(
    args: &CpgInputArgs,
    demand: Demand,
    order: Order,
) -> Result<Received, InputError> {
    let pool = pool(demand, order, (&|_| {}, &keep_records));
    if args.stdin || args.input_fd.is_some() || args.capture.is_some() {
        return Request {
            directory: args.capture.as_deref(),
            input_fd: args.input_fd,
            pool,
        }
        .receive()
        .into_received();
    }
    listed(&args.documents)
        .and_then(|listed| records_on_disk(listed, Listed::read, &pool))
        .map(|records| Received {
            records,
            receipt: None,
        })
        .map_err(InputError::Documents)
}

fn workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(MAX_WORKERS)
}

/// A pool that stops nothing on a failure: whatever it reads has already been written.
fn pool<'a>(demand: Demand, order: Order, (see, take): (See<'a>, OnRecords<'a>)) -> Pool<'a> {
    Pool {
        workers: workers(),
        demand,
        on_failure: &|| {},
        on_document: see,
        on_records: take,
        order,
    }
}

/// A frontend's stream, read by one thread and derived by a pool.
pub(crate) struct Stream<'a> {
    pub demand: Demand,
    pub on_failure: &'a (dyn Fn() + Sync),
    pub store: Option<Store<'a>>,
    /// Given each document's records as the pool derives them.
    pub on_records: OnRecords<'a>,
}

/// Where the received bytes are also sealed, so a frontend needs no storage code of its own.
pub(crate) struct Store<'a> {
    pub directory: &'a Path,
    pub encoding: Encoding,
}

impl Store<'_> {
    fn save(&self, index: usize, bytes: &[u8]) -> Result<SealedDocument, String> {
        let name = format!("document-{index:06}{}", self.encoding.suffix());
        transport::publish_parsed(&self.directory.join(name), self.encoding, bytes)
    }

    fn seal(&self, mut seals: Vec<(usize, SealedDocument)>) -> Result<(), String> {
        seals.sort_by_key(|(index, _)| *index);
        let seals = seals.into_iter().map(|(_, seal)| seal).collect();
        transport::write_json(
            &self.directory.join(transport::DOCUMENTS_MANIFEST),
            Encoding::Json,
            &DocumentManifest::of(seals),
        )
        .map(|_| ())
    }
}

/// Every record of a complete stream. The manifest is written only once the stream is.
pub(crate) fn stream_records<R: BufRead>(
    reader: &mut SequenceReader<R>,
    stream: &Stream<'_>,
) -> Result<Vec<Record>, String> {
    let pool = Pool {
        on_failure: stream.on_failure,
        ..pool(
            stream.demand,
            Order::Canonical,
            (&|_| {}, stream.on_records),
        )
    };
    pooled_stream(reader, stream.store.as_ref(), &pool)
}

/// Every record of a complete stream, derived on `pool`, each document also kept in `store`.
// rstyle-allow(side_effect_density): IO boundary: reads a stream to its end and keeps each document
fn pooled_stream<R: BufRead>(
    reader: &mut SequenceReader<R>,
    store: Option<&Store<'_>>,
    pool: &Pool<'_>,
) -> Result<Vec<Record>, String> {
    let seals = Mutex::new(Vec::new());
    let load = |index: usize, text: String| {
        let document = streamed(index + 1, &text)?;
        if let Some(store) = store {
            let seal = store.save(index, text.as_bytes())?;
            seals
                .lock()
                .expect("no panic while keeping")
                .push((index, seal));
        }
        Ok(document)
    };
    let jobs = std::iter::from_fn(|| reader.next_text().transpose());
    let records = cpg::pooled_records(jobs, load, pool)?;

    reader.complete()?;
    if let Some(store) = store {
        store.seal(seals.into_inner().expect("no panic while keeping"))?;
    }
    Ok(records)
}

/// The `number`th document of a stream, counting from 1.
fn streamed(number: usize, text: &str) -> Result<Parsed, String> {
    layout::parse(text).map_err(|error| format!("CPG stream document {number}: {error}"))
}

/// Every record of documents already on disk, read and derived on `pool`.
fn records_on_disk<J: Send>(
    jobs: impl IntoIterator<Item = J>,
    read: impl Fn(J) -> Result<Parsed, String> + Sync,
    pool: &Pool<'_>,
) -> Result<Vec<Record>, String> {
    let jobs = jobs.into_iter().map(Ok);
    cpg::pooled_records(jobs, |_, job| read(job), pool)
}

/// What's shown each document as it's read, for what a run's records don't hold.
pub(crate) type See<'a> = &'a (dyn Fn(&rstyle_core::cpg::model::Document) + Sync);

/// Every record of explicitly listed documents, each one shown to `see` and handed to `take`.
pub(crate) fn seen_file_records(
    paths: &[PathBuf],
    demand: Demand,
    hooks: (See<'_>, OnRecords<'_>),
) -> Result<Vec<Record>, String> {
    let pool = pool(demand, Order::Canonical, hooks);
    records_on_disk(listed(paths)?, Listed::read, &pool)
}

/// A listed document, and its seal when a manifest beside it seals the documents there.
struct Listed<'a> {
    path: &'a Path,
    seal: Option<SealedDocument>,
}

impl Listed<'_> {
    fn read(self) -> Result<Parsed, String> {
        let text = match &self.seal {
            Some(seal) => transport::read_sealed_text(directory(self.path), seal)?,
            None => transport::read_json_text(self.path)?,
        };
        layout::parse(&text).map_err(|error| format!("{}: {error}", self.path.display()))
    }
}

fn directory(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}

/// Each listed document with its seal. A directory's manifest is read once, however many of
/// its documents are listed.
fn listed(paths: &[PathBuf]) -> Result<Vec<Listed<'_>>, String> {
    let mut manifests = BTreeMap::new();
    for path in paths {
        if let Entry::Vacant(entry) = manifests.entry(directory(path)) {
            entry.insert(Seals::beside(path)?);
        }
    }
    paths
        .iter()
        .map(|path| {
            let seal = manifests[directory(path)].of(path)?;
            Ok(Listed { path, seal })
        })
        .collect()
}

/// The seals of a directory's manifest by file, if it has one.
struct Seals(Option<BTreeMap<String, SealedDocument>>);

impl Seals {
    // rstyle-allow(side_effect_density): IO boundary: reads a directory's document manifest
    fn beside(path: &Path) -> Result<Self, String> {
        let manifest = directory(path).join(transport::DOCUMENTS_MANIFEST);
        if !manifest.exists() {
            return Ok(Self(None));
        }
        let bytes = std::fs::read(&manifest).map_err(|e| e.to_string())?;
        let manifest = DocumentManifest::read(&bytes)?;
        let seals = manifest
            .documents
            .into_iter()
            .map(|seal| (seal.file.clone(), seal));
        Ok(Self(Some(seals.collect())))
    }

    fn of(&self, path: &Path) -> Result<Option<SealedDocument>, String> {
        let Some(seals) = &self.0 else {
            return Ok(None);
        };
        path.file_name()
            .and_then(OsStr::to_str)
            .and_then(|file| seals.get(file))
            .map(|seal| Some(seal.clone()))
            .ok_or_else(|| format!("{} is not listed in its document manifest", path.display()))
    }
}

/// Every record of a sealed capture, each document verified against its seal on a worker and
/// shown to `see`.
// rstyle-allow(side_effect_density): IO boundary: reads a capture's manifest
pub(crate) fn seen_capture_records(
    directory: &Path,
    demand: Demand,
    (see, take): (See<'_>, OnRecords<'_>),
) -> Result<Vec<Record>, String> {
    let manifest = DocumentManifest::read(&transport::read_manifest_bytes(directory)?)?;
    if manifest.documents.is_empty() {
        return Err("A completed capture must contain at least one document".into());
    }
    let seals = manifest.documents.iter().map(|seal| (directory, seal));
    seen_sealed_records(seals, demand, (see, take))
}

/// Every record of these sealed documents, wherever each one is, read on one pool.
pub(crate) fn sealed_records<'a>(
    seals: impl IntoIterator<Item = (&'a Path, &'a SealedDocument)>,
    demand: Demand,
    take: OnRecords<'_>,
) -> Result<Vec<Record>, String> {
    seen_sealed_records(seals, demand, (&|_| {}, take))
}

// rstyle-allow(side_effect_density): IO boundary: reads each sealed document on a worker
fn seen_sealed_records<'a>(
    seals: impl IntoIterator<Item = (&'a Path, &'a SealedDocument)>,
    demand: Demand,
    hooks: (See<'_>, OnRecords<'_>),
) -> Result<Vec<Record>, String> {
    let read = |(directory, seal): (&Path, &SealedDocument)| {
        transport::read_sealed_text(directory, seal)
            .and_then(|text| layout::parse(&text))
            .map_err(|error| format!("{}: {error}", seal.file))
    };
    records_on_disk(seals, read, &pool(demand, Order::Canonical, hooks))
}

pub(super) fn receive(args: &CpgReceiveArgs, format: Format) -> Result<(String, u8), String> {
    let outcome = Request {
        directory: args.capture.as_deref(),
        input_fd: args.input_fd,
        pool: pool(Demand::default(), Order::Arrival, (&|_| {}, &discard)),
    }
    .receive();
    outcome.receipt.output(format)
}

/// Keeps no record: a receipt says only whether every document could be derived.
fn discard(records: Vec<Record>) -> Result<Vec<Record>, String> {
    drop(records);
    Ok(Vec::new())
}

/// A capture or a stream, read by one thread, which verifies its transport in order, and
/// derived on a pool.
struct Request<'a> {
    directory: Option<&'a Path>,
    input_fd: Option<i64>,
    pool: Pool<'a>,
}

impl Request<'_> {
    fn receive(self) -> Outcome {
        match self.directory {
            Some(directory) => self.capture(directory),
            None => self.stream(),
        }
    }

    // rstyle-allow(side_effect_density): IO boundary: opens stdin or an inherited descriptor
    fn stream(self) -> Outcome {
        let Some(number) = self.input_fd else {
            let reader = SequenceReader::new(std::io::stdin().lock());
            return self.stream_session(reader, Receipt::stdin(self.pool.demand));
        };
        let receipt = Receipt::descriptor(number, self.pool.demand);
        match super::descriptor::open(number) {
            Ok(file) => {
                let reader = SequenceReader::new(BufReader::with_capacity(STREAM_BUFFER, file));
                self.stream_session(reader, receipt)
            }
            Err(error) => Outcome::finished(receipt, None, Err(error)),
        }
    }

    fn stream_session<R: BufRead>(
        &self,
        mut reader: SequenceReader<R>,
        receipt: Receipt,
    ) -> Outcome {
        let result = pooled_stream(&mut reader, None, &self.pool);
        let progress = TransportProgress::Stream(reader.progress());
        Outcome::finished(receipt, Some(progress), result)
    }

    // rstyle-allow(side_effect_density): IO boundary: reads a capture to its end
    fn capture(self, directory: &Path) -> Outcome {
        let receipt = Receipt::new(directory, self.pool.demand);
        let mut reader = match CaptureReader::open(directory) {
            Ok(reader) => reader,
            Err(error) => return Outcome::finished(receipt, None, Err(error)),
        };
        let jobs = std::iter::from_fn(|| reader.next_text().transpose());
        let result = cpg::pooled_records(jobs, |_, document| captured(&document), &self.pool)
            .and_then(|records| reader.complete().map(|()| records));
        let progress = TransportProgress::Capture(reader.progress().clone());
        Outcome::finished(receipt, Some(progress), result)
    }
}

fn captured(document: &Captured) -> Result<Parsed, String> {
    layout::parse(&document.text).map_err(|error| format!("{}: {error}", document.file))
}

struct Outcome {
    records: Vec<Record>,
    receipt: Receipt,
}

impl Outcome {
    fn finished(
        receipt: Receipt,
        progress: Option<TransportProgress>,
        result: Result<Vec<Record>, String>,
    ) -> Self {
        let error = result.as_ref().err().cloned();
        let receipt = receipt.finished(progress, error);
        Self {
            records: result.unwrap_or_default(),
            receipt,
        }
    }

    fn into_received(self) -> Result<Received, InputError> {
        match self.receipt.error() {
            Some(_) => Err(InputError::Rejected(Box::new(self.receipt))),
            None => Ok(Received {
                records: self.records,
                receipt: Some(self.receipt),
            }),
        }
    }
}
