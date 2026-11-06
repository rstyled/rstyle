//! Files, captures, and stdin share incremental derivation and receipt handling.
use super::receipt::{Receipt, TransportProgress};
use crate::{CpgInputArgs, CpgReceiveArgs, Format};
use rstyle_core::cpg::{
    self,
    records::{Demand, Record},
    transport::{CaptureReader, SequenceReader},
};
use std::io::BufRead;
use std::path::Path;

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
    if args.stdin || args.capture.is_some() {
        return Request::new(args.capture.as_deref(), demand, true)
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

pub(super) fn receive(args: &CpgReceiveArgs, format: Format) -> Result<(String, u8), String> {
    let outcome = Request::new(args.capture.as_deref(), Demand::default(), false).receive();
    outcome.receipt.output(format)
}

struct Request<'a> {
    directory: Option<&'a Path>,
    demand: Demand,
    retain: bool,
}

impl<'a> Request<'a> {
    fn new(directory: Option<&'a Path>, demand: Demand, retain: bool) -> Self {
        Self {
            directory,
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
        let reader = SequenceReader::new(std::io::stdin().lock());
        let receipt = Receipt::stdin(self.demand);
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
