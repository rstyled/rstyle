//! Files, captures, and pipes share incremental derivation and receipt handling.
use super::receipt::{Receipt, TransportProgress};
use crate::{CpgInputArgs, CpgReceiveArgs, Format};
use rstyle_core::cpg::{
    self, Pool,
    records::{Demand, Record},
    transport::{self, CaptureReader, DocumentManifest, Encoding, SealedDocument, SequenceReader},
};
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

pub(super) fn read(args: &CpgInputArgs, demand: Demand) -> Result<Received, InputError> {
    if args.stdin || args.input_fd.is_some() || args.capture.is_some() {
        return Request::new(args.capture.as_deref(), args.input_fd, demand, true)
            .receive()
            .into_received();
    }
    files(&args.documents, demand)
}

fn files(paths: &[std::path::PathBuf], demand: Demand) -> Result<Received, InputError> {
    cpg::read_records(paths.iter().map(|path| super::read(path)), demand)
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

/// Reads as `read` does, deriving documents on a worker pool.
// rstyle-allow(side_effect_density): IO boundary: reads whichever input was selected
pub(super) fn read_pooled(args: &CpgInputArgs, demand: Demand) -> Result<Received, InputError> {
    if args.capture.is_some() {
        return read(args, demand);
    }
    if args.stdin || args.input_fd.is_some() {
        return pooled_stream(args.input_fd, demand).into_received();
    }
    file_records(&args.documents, demand)
        .map(|records| Received {
            records,
            receipt: None,
        })
        .map_err(InputError::Documents)
}

// rstyle-allow(side_effect_density): IO boundary: opens stdin or an inherited descriptor
fn pooled_stream(input_fd: Option<i64>, demand: Demand) -> Outcome {
    let Some(number) = input_fd else {
        let reader = SequenceReader::new(std::io::stdin().lock());
        return pooled_session(reader, Receipt::stdin(demand), demand);
    };
    let receipt = Receipt::descriptor(number, demand);
    match super::descriptor::open(number) {
        Ok(file) => {
            let reader = SequenceReader::new(BufReader::with_capacity(STREAM_BUFFER, file));
            pooled_session(reader, receipt, demand)
        }
        Err(error) => Outcome::finished(receipt, None, Err(error)),
    }
}

// rstyle-allow(side_effect_density): IO boundary: reads the stream to its end
fn pooled_session<R: BufRead>(
    mut reader: SequenceReader<R>,
    receipt: Receipt,
    demand: Demand,
) -> Outcome {
    let stream = Stream {
        demand,
        on_failure: &|| {},
        store: None,
    };
    let result = stream_records(&mut reader, &stream);
    let progress = TransportProgress::Stream(reader.progress());
    Outcome::finished(receipt, Some(progress), result)
}

/// A frontend's stream, read by one thread and derived by a pool.
pub(crate) struct Stream<'a> {
    pub demand: Demand,
    pub on_failure: &'a (dyn Fn() + Sync),
    pub store: Option<Store<'a>>,
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
    let seals = Mutex::new(Vec::new());
    let load = |index: usize, bytes: Vec<u8>| {
        let document = serde_json::from_slice(&bytes)
            .map_err(|error| format!("CPG stream document {}: {error}", index + 1))?;
        if let Some(store) = &stream.store {
            let seal = store.save(index, &bytes)?;
            seals
                .lock()
                .expect("no panic while keeping")
                .push((index, seal));
        }
        Ok(document)
    };
    let pool = Pool {
        workers: workers(),
        demand: stream.demand,
        on_failure: stream.on_failure,
    };
    let jobs = std::iter::from_fn(|| reader.next_bytes().transpose());
    let records = cpg::pooled_records(jobs, load, &pool)?;

    reader.complete()?;
    if let Some(store) = &stream.store {
        store.seal(seals.into_inner().expect("no panic while keeping"))?;
    }
    Ok(records)
}

/// Every record of documents already on disk, read and derived on a pool. There's nothing to
/// stop if one fails.
fn records_on_disk<J: Send>(
    jobs: impl IntoIterator<Item = J>,
    read: impl Fn(J) -> Result<serde_json::Value, String> + Sync,
    demand: Demand,
) -> Result<Vec<Record>, String> {
    let pool = Pool {
        workers: workers(),
        demand,
        on_failure: &|| {},
    };
    let jobs = jobs.into_iter().map(Ok);
    cpg::pooled_records(jobs, |_, job| read(job), &pool)
}

/// Every record of explicitly listed documents.
pub(crate) fn file_records(paths: &[PathBuf], demand: Demand) -> Result<Vec<Record>, String> {
    records_on_disk(paths, |path| super::read(path), demand)
}

/// Every record of a sealed capture, each document verified against its seal on a worker.
pub(crate) fn capture_records(directory: &Path, demand: Demand) -> Result<Vec<Record>, String> {
    let manifest = DocumentManifest::read(&transport::read_manifest_bytes(directory)?)?;
    if manifest.documents.is_empty() {
        return Err("A completed capture must contain at least one document".into());
    }
    let seals = manifest.documents.iter().map(|seal| (directory, seal));
    sealed_records(seals, demand)
}

/// Every record of these sealed documents, wherever each one is, read on one pool.
pub(crate) fn sealed_records<'a>(
    seals: impl IntoIterator<Item = (&'a Path, &'a SealedDocument)>,
    demand: Demand,
) -> Result<Vec<Record>, String> {
    let read = |(directory, seal): (&Path, &SealedDocument)| {
        transport::read_sealed(directory, seal).map_err(|error| format!("{}: {error}", seal.file))
    };
    records_on_disk(seals, read, demand)
}

pub(super) fn receive(args: &CpgReceiveArgs, format: Format) -> Result<(String, u8), String> {
    let outcome = Request::new(
        args.capture.as_deref(),
        args.input_fd,
        Demand::default(),
        false,
    )
    .receive();
    outcome.receipt.output(format)
}

struct Request<'a> {
    directory: Option<&'a Path>,
    input_fd: Option<i64>,
    demand: Demand,
    retain: bool,
}

impl<'a> Request<'a> {
    fn new(
        directory: Option<&'a Path>,
        input_fd: Option<i64>,
        demand: Demand,
        retain: bool,
    ) -> Self {
        Self {
            directory,
            input_fd,
            demand,
            retain,
        }
    }

    fn receive(self) -> Outcome {
        match self.directory {
            Some(directory) => self.capture(directory),
            None => self.stream(),
        }
    }

    fn stream(self) -> Outcome {
        match self.input_fd {
            Some(number) => self.descriptor(number),
            None => self.stdin(),
        }
    }

    fn stdin(self) -> Outcome {
        let reader = SequenceReader::new(std::io::stdin().lock());
        let receipt = Receipt::stdin(self.demand);
        Session::new(reader, receipt, self.demand, self.retain).run()
    }

    fn descriptor(self, number: i64) -> Outcome {
        let receipt = Receipt::descriptor(number, self.demand);
        match super::descriptor::open(number) {
            Ok(file) => self.descriptor_session(file, receipt),
            Err(error) => Outcome::finished(receipt, None, Err(error)),
        }
    }

    fn descriptor_session(self, file: std::fs::File, receipt: Receipt) -> Outcome {
        let reader = SequenceReader::new(BufReader::new(file));
        Session::new(reader, receipt, self.demand, self.retain).run()
    }

    fn capture(self, directory: &Path) -> Outcome {
        let receipt = Receipt::new(directory, self.demand);
        let reader = CaptureReader::open(directory);
        self.opened(receipt, reader)
    }

    fn opened(self, receipt: Receipt, reader: Result<CaptureReader, String>) -> Outcome {
        match reader {
            Ok(reader) => Session::new(reader, receipt, self.demand, self.retain).run(),
            Err(error) => Outcome::finished(receipt, None, Err(error)),
        }
    }
}

trait Source: Iterator<Item = Result<serde_json::Value, String>> {
    fn complete(&self) -> Result<(), String>;
    fn progress(&self) -> TransportProgress;
}

impl Source for CaptureReader {
    fn complete(&self) -> Result<(), String> {
        CaptureReader::complete(self)
    }
    fn progress(&self) -> TransportProgress {
        TransportProgress::Capture(CaptureReader::progress(self).clone())
    }
}

impl<R: BufRead> Source for SequenceReader<R> {
    fn complete(&self) -> Result<(), String> {
        SequenceReader::complete(self)
    }
    fn progress(&self) -> TransportProgress {
        TransportProgress::Stream(SequenceReader::progress(self))
    }
}

struct Session<R> {
    reader: R,
    receipt: Receipt,
    demand: Demand,
    retain: bool,
}

impl<R: Source> Session<R> {
    fn new(reader: R, receipt: Receipt, demand: Demand, retain: bool) -> Self {
        Self {
            reader,
            receipt,
            demand,
            retain,
        }
    }

    fn run(mut self) -> Outcome {
        let result = self
            .read()
            .and_then(|records| self.reader.complete().map(|()| records));
        Outcome::finished(self.receipt, Some(self.reader.progress()), result)
    }

    fn read(&mut self) -> Result<Vec<Record>, String> {
        let retain = self.retain;
        let mut records = Vec::new();

        cpg::visit_records(&mut self.reader, self.demand, |batch| {
            if retain {
                records.extend(batch);
            }
            Ok(())
        })?;
        Ok(records)
    }
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
