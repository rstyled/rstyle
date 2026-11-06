//! A receipt records transport verification separately from requested CPG validation.
use crate::Format;
use rstyle_core::cpg::{
    layout,
    records::Demand,
    transport::{CaptureProgress, SequenceProgress},
};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Accepted,
    Rejected,
}

#[derive(Serialize)]
struct Validation {
    profile: &'static str,
    version: &'static str,
    graphs: bool,
    runtime_bounds: bool,
}

#[derive(Serialize)]
struct Failure {
    message: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum TransportProgress {
    Capture(CaptureProgress),
    Stream(SequenceProgress),
}

#[derive(Serialize)]
pub(super) struct Receipt {
    schema: &'static str,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    capture: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'static str>,
    validation: Validation,
    transport: Option<TransportProgress>,
    failures: Vec<Failure>,
}

impl Receipt {
    pub(super) fn new(capture: &Path, demand: Demand) -> Self {
        Self::request(Some(capture), demand)
    }

    pub(super) fn stdin(demand: Demand) -> Self {
        Self::request(None, demand)
    }

    fn request(capture: Option<&Path>, demand: Demand) -> Self {
        Self {
            schema: match capture {
                Some(_) => "rstyle.cpg-receipt/1",
                None => "rstyle.cpg-stream-receipt/1",
            },
            status: Status::Rejected,
            capture: capture.map(PathBuf::from),
            source: capture.is_none().then_some("stdin"),
            validation: Validation {
                profile: layout::PROFILE,
                version: layout::VERSION,
                graphs: demand.graphs,
                runtime_bounds: demand.bounds,
            },
            transport: None,
            failures: Vec::new(),
        }
    }

    pub(super) fn finished(
        self,
        transport: Option<TransportProgress>,
        error: Option<String>,
    ) -> Self {
        let status = if error.is_some() {
            Status::Rejected
        } else {
            Status::Accepted
        };
        let failures = error
            .into_iter()
            .map(|message| Failure { message })
            .collect();
        Self {
            status,
            transport,
            failures,
            ..self
        }
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.failures
            .first()
            .map(|failure| failure.message.as_str())
    }

    pub(super) fn output(&self, format: Format) -> Result<(String, u8), String> {
        let text = match format {
            Format::Json => serde_json::to_string_pretty(self).map_err(|e| e.to_string())?,
            Format::Text => self.text(),
        };

        Ok((text, if self.error().is_some() { 2 } else { 0 }))
    }

    fn text(&self) -> String {
        match self.error() {
            Some(error) => format!("CPG {} rejected: {error}", self.kind()),
            None => self.accepted_text(),
        }
    }

    fn kind(&self) -> &str {
        if self.source.is_some() {
            "stream"
        } else {
            "capture"
        }
    }

    fn accepted_text(&self) -> String {
        match &self.transport {
            Some(TransportProgress::Capture(progress)) => format!(
                "CPG capture accepted: {} documents; {} stored bytes, {} decoded bytes verified",
                progress.verified_documents, progress.stored_bytes, progress.decoded_bytes
            ),
            Some(TransportProgress::Stream(progress)) => format!(
                "CPG stream accepted: {} documents; {} uncompressed bytes received",
                progress.received_documents, progress.received_bytes
            ),
            None => "CPG capture has no transport verification".into(),
        }
    }
}

pub(super) fn text(report: String, receipt: &Option<Receipt>) -> String {
    match receipt {
        Some(receipt) => format!("{report}\n{}", receipt.text()),
        None => report,
    }
}

pub(super) fn json(report: &impl Serialize, receipt: &Option<Receipt>) -> Result<String, String> {
    match receipt {
        Some(receipt) => with_receipt(report, receipt),
        None => serde_json::to_string_pretty(report).map_err(|e| e.to_string()),
    }
}

fn with_receipt(report: &impl Serialize, receipt: &Receipt) -> Result<String, String> {
    let mut report = serde_json::to_value(report).map_err(|e| e.to_string())?;
    let object = report
        .as_object_mut()
        .ok_or("The CPG report must be a JSON object")?;

    object.insert(
        "capture_receipt".into(),
        serde_json::to_value(receipt).map_err(|e| e.to_string())?,
    );
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}
