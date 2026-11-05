mod codecs;
mod model;
mod normalize;
mod original;

// Use the real reader's types without importing the analysis engine.
#[allow(dead_code)]
#[rustfmt::skip]
#[path = "../../../crates/rstyle-core/src/cpg/model.rs"]
mod cpg_model;

#[allow(warnings)]
#[rustfmt::skip]
mod archive_generated;

use clap::{Parser, Subcommand, ValueEnum};
use codecs::Codec;
use model::Archive;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Hash the canonical content of each original document before timed trials.
    OriginalInventory {
        #[arg(long)]
        out: PathBuf,
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
    },
    /// Encode or fully decode original JSON documents without the shared arena.
    OriginalTrial {
        #[arg(long, value_enum)]
        operation: Operation,
        #[arg(long)]
        wire: PathBuf,
        #[arg(long)]
        result: PathBuf,
        #[arg(long)]
        zstd_level: Option<i32>,
        #[arg(long)]
        generic_json: bool,
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
    },
    /// Normalize documents, prove lossless reconstruction, and freeze a shared model.
    Prepare {
        #[arg(long)]
        out: PathBuf,
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
    },
    /// One process-isolated timed codec operation. I/O and validation are separate.
    Trial {
        #[arg(long, value_enum)]
        codec: Codec,
        #[arg(long, value_enum)]
        operation: Operation,
        #[arg(long)]
        model: PathBuf,
        #[arg(long)]
        wire: PathBuf,
        #[arg(long)]
        result: PathBuf,
        #[arg(long)]
        zstd_level: Option<i32>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Operation {
    Encode,
    Decode,
}

#[derive(Serialize)]
struct Trial {
    representation: &'static str,
    codec: Codec,
    operation: Operation,
    zstd_level: Option<i32>,
    seconds: f64,
    codec_seconds: f64,
    compression_seconds: f64,
    io_seconds: f64,
    validation_seconds: f64,
    wire_bytes: usize,
    uncompressed_bytes: usize,
    archive_digest: String,
    wire_sha256: String,
    baseline_rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
    peak_rss_reset: bool,
}

fn main() -> Result<(), String> {
    match Cli::parse().command {
        Command::OriginalInventory { out, inputs } => {
            write_json(&out, &original::inventory(&inputs)?)
        }
        Command::OriginalTrial {
            operation,
            wire,
            result,
            zstd_level,
            generic_json,
            inputs,
        } => {
            let model = if generic_json {
                original::ReadModel::Json
            } else {
                original::ReadModel::Cpg
            };
            let trial = original::trial(operation, &inputs, (&wire, model), zstd_level)?;
            write_json(&result, &trial)
        }
        Command::Prepare { out, inputs } => prepare(&out, &inputs),
        Command::Trial {
            codec,
            operation,
            model,
            wire,
            result,
            zstd_level,
        } => {
            let trial = trial(codec, operation, (&model, &wire), zstd_level)?;
            write_json(&result, &trial)
        }
    }
}

fn prepare(out: &Path, inputs: &[PathBuf]) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut normalizer = normalize::Normalizer::new();
    let mut inventory = Vec::new();
    for path in inputs {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        inventory.push(serde_json::json!({"path":path,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes))}));
        drop(bytes);
        eprintln!("Normalizing {}", path.display());
        normalizer.add(path)?;
    }
    let (archive, stats) = normalizer.finish()?;
    let normalization_seconds = start.elapsed().as_secs_f64();
    let digest = archive.digest();
    let start = Instant::now();
    let file = std::fs::File::create(out.join("model.json")).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &archive).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    let json_write_seconds = start.elapsed().as_secs_f64();
    write_json(
        &out.join("prepare.json"),
        &serde_json::json!({
            "archive_version":model::VERSION,"archive_digest":digest,"stats":stats,"inputs":inventory,
            "normalization_seconds":normalization_seconds,"json_write_seconds":json_write_seconds,
            "edges":archive.edges.len(),"model_json_bytes":std::fs::metadata(out.join("model.json")).map_err(|e|e.to_string())?.len(),
            "process_peak_rss_bytes":rss("VmHWM:"),"lossless_reconstruction":true,
        }),
    )
}

fn trial(
    codec: Codec,
    operation: Operation,
    (model, wire): (&Path, &Path),
    level: Option<i32>,
) -> Result<Trial, String> {
    // Warm codec code paths consistently before loading the measured input.
    let tiny = Archive::empty();
    for codec in [
        Codec::Json,
        Codec::Flatbuffers,
        Codec::Flexbuffers,
        Codec::Protobuf,
    ] {
        let encoded = codecs::encode(codec, &tiny)?;
        let decoded = codecs::decode(codec, &encoded)?;
        std::hint::black_box(decoded);
    }
    match operation {
        Operation::Encode => encode_trial(codec, model, wire, level),
        Operation::Decode => decode_trial(codec, wire, level),
    }
}

fn encode_trial(
    codec: Codec,
    model: &Path,
    wire: &Path,
    level: Option<i32>,
) -> Result<Trial, String> {
    let file = std::fs::File::open(model).map_err(|e| e.to_string())?;
    let archive: Archive =
        serde_json::from_reader(BufReader::new(file)).map_err(|e| e.to_string())?;
    let reset = reset_peak();
    let baseline = rss("VmRSS:");
    let start = Instant::now();
    let raw = codecs::encode(codec, std::hint::black_box(&archive))?;
    let codec_seconds = start.elapsed().as_secs_f64();
    let raw_len = raw.len();
    let compress = Instant::now();
    let bytes = match level {
        Some(level) => zstd::stream::encode_all(raw.as_slice(), level)
            .map(codecs::Buffer::from)
            .map_err(|e| e.to_string())?,
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
    std::fs::write(wire, &bytes).map_err(|e| e.to_string())?;
    let io_seconds = io.elapsed().as_secs_f64();
    let validation = Instant::now();
    let digest = archive.digest();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    Ok(Trial {
        representation: "shared-arena",
        codec,
        operation: Operation::Encode,
        zstd_level: level,
        seconds,
        codec_seconds,
        compression_seconds,
        io_seconds,
        validation_seconds: validation.elapsed().as_secs_f64(),
        wire_bytes: bytes.len(),
        uncompressed_bytes: raw_len,
        archive_digest: digest,
        wire_sha256: hash,
        baseline_rss_bytes: baseline,
        peak_rss_bytes: peak,
        peak_rss_reset: reset,
    })
}

fn decode_trial(codec: Codec, wire: &Path, level: Option<i32>) -> Result<Trial, String> {
    let io = Instant::now();
    let bytes = std::fs::read(wire).map_err(|e| e.to_string())?;
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
    let archive = codecs::decode(codec, std::hint::black_box(input))?;
    let codec_seconds = decode.elapsed().as_secs_f64();
    let seconds = start.elapsed().as_secs_f64();
    let peak = rss("VmHWM:");
    let validation = Instant::now();
    archive.validate()?;
    let digest = archive.digest();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    Ok(Trial {
        representation: "shared-arena",
        codec,
        operation: Operation::Decode,
        zstd_level: level,
        seconds,
        codec_seconds,
        compression_seconds,
        io_seconds,
        validation_seconds: validation.elapsed().as_secs_f64(),
        wire_bytes: bytes.len(),
        uncompressed_bytes: input.len(),
        archive_digest: digest,
        wire_sha256: hash,
        baseline_rss_bytes: baseline,
        peak_rss_bytes: peak,
        peak_rss_reset: reset,
    })
}

fn rss(field: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix(field)?
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()?
                .checked_mul(1024)
        })
}

fn reset_peak() -> bool {
    // Linux clear_refs=5 resets only this process's high-water RSS. This excludes
    // model-loading temporaries from the measured codec stage's memory peak.
    std::fs::write("/proc/self/clear_refs", b"5").is_ok()
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(BufWriter::new(file), value).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
