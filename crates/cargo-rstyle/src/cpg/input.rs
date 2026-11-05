//! Document inputs share incremental derivation; complete captures also get receipts.
use super::receipt::Receipt;
use crate::{CpgInputArgs, Format};
use rstyle_core::cpg::{
    self,
    records::{Demand, Record},
    transport::CaptureReader,
};
use std::path::Path;

pub(super) struct Received {
    pub records: Vec<Record>,
    pub receipt: Option<Receipt>,
}

pub(super) enum InputError {
    Documents(String),
    Capture(Box<Receipt>),
}

impl InputError {
    pub(super) fn output(self, format: Format) -> Result<(String, u8), String> {
        match self {
            Self::Capture(receipt) => receipt.output(format),
            Self::Documents(message) => Err(message),
        }
    }
}

pub(super) fn read(args: &CpgInputArgs, demand: Demand) -> Result<Received, InputError> {
    match &args.capture {
        Some(directory) => capture(directory, demand, true).into_received(),
        None => files(&args.documents, demand),
    }
}

fn files(paths: &[std::path::PathBuf], demand: Demand) -> Result<Received, InputError> {
    cpg::read_records(paths.iter().map(|path| super::read(path)), demand)
        .map(|records| Received {
            records,
            receipt: None,
        })
        .map_err(InputError::Documents)
}

pub(super) fn receive(directory: &Path, format: Format) -> Result<(String, u8), String> {
    capture(directory, Demand::default(), false)
        .receipt
        .output(format)
}

fn capture(directory: &Path, demand: Demand, retain: bool) -> Outcome {
    let receipt = Receipt::new(directory, demand);
    match CaptureReader::open(directory) {
        Ok(reader) => Session {
            reader,
            receipt,
            demand,
            retain,
        }
        .run(),
        Err(error) => Outcome::finished(receipt, None, Err(error)),
    }
}

struct Session {
    reader: CaptureReader,
    receipt: Receipt,
    demand: Demand,
    retain: bool,
}

impl Session {
    fn run(mut self) -> Outcome {
        let result = self
            .read()
            .and_then(|records| self.reader.complete().map(|()| records));
        Outcome::finished(self.receipt, Some(self.reader.progress().clone()), result)
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
        progress: Option<rstyle_core::cpg::transport::CaptureProgress>,
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
            Some(_) => Err(InputError::Capture(Box::new(self.receipt))),
            None => Ok(Received {
                records: self.records,
                receipt: Some(self.receipt),
            }),
        }
    }
}
