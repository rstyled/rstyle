//! Original JSON: the producer's Value encoder and the actual CPG Document reader.
//! Documents keep their existing boundaries. Timings sum the codec work over
//! all documents; loading, I/O, validation, and destruction stay outside them.

use crate::{Operation, Trial, codecs::Codec, reset_peak, rss};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Clone, Copy)]
pub enum ReadModel {
    Cpg,
    Json,
}

enum Decoded {
    Cpg(Box<crate::cpg_model::Document>),
    Json(Value),
}

impl Decoded {
    fn read(input: &[u8], model: ReadModel) -> Result<Self, String> {
        match model {
            ReadModel::Cpg => serde_json::from_slice(input)
                .map(|value| Self::Cpg(Box::new(value)))
                .map_err(|e| e.to_string()),
            ReadModel::Json => serde_json::from_slice(input)
                .map(Self::Json)
                .map_err(|e| e.to_string()),
        }
    }

    fn digest(self, input: &[u8]) -> Result<[u8; 32], String> {
        match self {
            Self::Json(value) => document_digest(&value),
            Self::Cpg(value) => {
                std::hint::black_box(&value);
                drop(value);
                // Validate canonical content outside the codec interval, without
                // retaining both the typed reader output and a generic JSON tree.
                let value: Value = serde_json::from_slice(input).map_err(|e| e.to_string())?;
                document_digest(&value)
            }
        }
    }
}

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn document_digest(document: &Value) -> Result<[u8; 32], String> {
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, document).map_err(|e| e.to_string())?;
    Ok(writer.0.finalize().into())
}

fn document_set_digest(digests: &[[u8; 32]]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"cpg-storage-original-documents-v1\0");
    hash.update((digests.len() as u64).to_le_bytes());
    for digest in digests {
        hash.update(digest);
    }
    format!("{:x}", hash.finalize())
}

#[derive(Serialize)]
pub struct Inventory {
    pub original_digest: String,
    document_digests: Vec<String>,
    representation: &'static str,
}

pub fn inventory(paths: &[PathBuf]) -> Result<Inventory, String> {
    let mut digests = Vec::new();
    for path in paths {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let document: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        drop(bytes);
        digests.push(document_digest(&document)?);
    }
    Ok(Inventory {
        original_digest: document_set_digest(&digests),
        document_digests: digests
            .iter()
            .map(|d| d.iter().map(|b| format!("{b:02x}")).collect())
            .collect(),
        representation: "original-json-dom",
    })
}

pub fn trial(
    operation: Operation,
    paths: &[PathBuf],
    (wire, model): (&Path, ReadModel),
    level: Option<i32>,
) -> Result<Trial, String> {
    let tiny = serde_json::json!({"warmup": [null, true, 42, "value"]});
    let encoded = serde_json::to_vec(&tiny).map_err(|e| e.to_string())?;
    std::hint::black_box(serde_json::from_slice::<Value>(&encoded).map_err(|e| e.to_string())?);
    if matches!(model, ReadModel::Cpg) {
        std::hint::black_box(Decoded::read(
            include_bytes!("../../../crates/rstyle-core/src/cpg/testdata/example.cpg.json"),
            model,
        )?);
    }
    std::fs::create_dir_all(wire).map_err(|e| e.to_string())?;
    let mut result = Trial {
        representation: match model {
            ReadModel::Cpg => "original-cpg-document",
            ReadModel::Json => "original-json-dom",
        },
        codec: Codec::Json,
        operation,
        zstd_level: level,
        seconds: 0.0,
        codec_seconds: 0.0,
        compression_seconds: 0.0,
        io_seconds: 0.0,
        validation_seconds: 0.0,
        wire_bytes: 0,
        uncompressed_bytes: 0,
        archive_digest: String::new(),
        wire_sha256: String::new(),
        baseline_rss_bytes: None,
        peak_rss_bytes: None,
        peak_rss_reset: true,
    };
    let mut document_hashes = Vec::new();
    let mut wire_hashes = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let target = wire.join(format!("document-{index:06}.bin"));
        let observed = match operation {
            Operation::Encode => encode(path, &target, level)?,
            Operation::Decode => decode(&target, level, model)?,
        };
        result.seconds += observed.seconds;
        result.codec_seconds += observed.codec_seconds;
        result.compression_seconds += observed.compression_seconds;
        result.io_seconds += observed.io_seconds;
        result.validation_seconds += observed.validation_seconds;
        result.wire_bytes += observed.wire_bytes;
        result.uncompressed_bytes += observed.uncompressed_bytes;
        result.baseline_rss_bytes = result.baseline_rss_bytes.max(observed.baseline_rss_bytes);
        result.peak_rss_bytes = result.peak_rss_bytes.max(observed.peak_rss_bytes);
        result.peak_rss_reset &= observed.peak_rss_reset;
        document_hashes.push(observed.document_digest);
        wire_hashes.push(observed.wire_digest);
    }
    result.archive_digest = document_set_digest(&document_hashes);
    result.wire_sha256 = document_set_digest(&wire_hashes);
    Ok(result)
}

struct Observation {
    seconds: f64,
    codec_seconds: f64,
    compression_seconds: f64,
    io_seconds: f64,
    validation_seconds: f64,
    wire_bytes: usize,
    uncompressed_bytes: usize,
    baseline_rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
    peak_rss_reset: bool,
    document_digest: [u8; 32],
    wire_digest: [u8; 32],
}

fn encode(path: &Path, target: &Path, level: Option<i32>) -> Result<Observation, String> {
    let input = std::fs::read(path).map_err(|e| e.to_string())?;
    let document: Value = serde_json::from_slice(&input).map_err(|e| e.to_string())?;
    drop(input);
    let reset = reset_peak();
    let baseline = rss("VmRSS:");
    let start = Instant::now();
    let raw = serde_json::to_vec(std::hint::black_box(&document)).map_err(|e| e.to_string())?;
    let codec_seconds = start.elapsed().as_secs_f64();
    let uncompressed_bytes = raw.len();
    let compress = Instant::now();
    let bytes = match level {
        Some(level) => {
            zstd::stream::encode_all(raw.as_slice(), level).map_err(|e| e.to_string())?
        }
        None => raw,
    };
    let compression_seconds = if level.is_some() {
        compress.elapsed().as_secs_f64()
    } else {
        0.0
    };
    let seconds = start.elapsed().as_secs_f64();
    let peak = rss("VmHWM:");
    let io = Instant::now();
    std::fs::write(target, &bytes).map_err(|e| e.to_string())?;
    let io_seconds = io.elapsed().as_secs_f64();
    let validation = Instant::now();
    let document_digest = document_digest(&document)?;
    let wire_digest = Sha256::digest(&bytes).into();
    Ok(Observation {
        seconds,
        codec_seconds,
        compression_seconds,
        io_seconds,
        validation_seconds: validation.elapsed().as_secs_f64(),
        wire_bytes: bytes.len(),
        uncompressed_bytes,
        baseline_rss_bytes: baseline,
        peak_rss_bytes: peak,
        peak_rss_reset: reset,
        document_digest,
        wire_digest,
    })
}

fn decode(target: &Path, level: Option<i32>, model: ReadModel) -> Result<Observation, String> {
    let io = Instant::now();
    let bytes = std::fs::read(target).map_err(|e| e.to_string())?;
    let io_seconds = io.elapsed().as_secs_f64();
    let reset = reset_peak();
    let baseline = rss("VmRSS:");
    let start = Instant::now();
    let raw = match level {
        Some(_) => zstd::stream::decode_all(bytes.as_slice()).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    let compression_seconds = if level.is_some() {
        start.elapsed().as_secs_f64()
    } else {
        0.0
    };
    let input = if level.is_some() {
        raw.as_slice()
    } else {
        bytes.as_slice()
    };
    let decode = Instant::now();
    let document = Decoded::read(std::hint::black_box(input), model)?;
    let codec_seconds = decode.elapsed().as_secs_f64();
    let seconds = start.elapsed().as_secs_f64();
    let peak = rss("VmHWM:");
    let validation = Instant::now();
    let document_digest = document.digest(input)?;
    let wire_digest = Sha256::digest(&bytes).into();
    Ok(Observation {
        seconds,
        codec_seconds,
        compression_seconds,
        io_seconds,
        validation_seconds: validation.elapsed().as_secs_f64(),
        wire_bytes: bytes.len(),
        uncompressed_bytes: input.len(),
        baseline_rss_bytes: baseline,
        peak_rss_bytes: peak,
        peak_rss_reset: reset,
        document_digest,
        wire_digest,
    })
}
