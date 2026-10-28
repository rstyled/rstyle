use super::model::Baseline;
use std::{fs::File, io::Read, path::Path};

const MAX_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn load(path: &Path) -> Result<Baseline, String> {
    let bytes = read_bounded(path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes = Vec::new();
    File::open(path).and_then(|file| file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))?;
    Ok(bytes)
}

fn parse(bytes: &[u8]) -> Result<Baseline, String> {
    bounded(bytes)?;
    let baseline: Baseline = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    baseline.validate()?;
    Ok(baseline)
}

pub(super) fn publish(path: &Path, baseline: &Baseline) -> Result<(), String> {
    baseline.validate()?;
    let bytes = serde_json::to_vec_pretty(baseline).map_err(|error| error.to_string())?;
    bounded(&bytes)?;

    super::super::publication::write(path, &bytes, super::super::publication::Mode::CreateNew)
}

fn bounded(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_BYTES {
        return Err("Density baseline exceeds the 4 MiB byte limit".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
