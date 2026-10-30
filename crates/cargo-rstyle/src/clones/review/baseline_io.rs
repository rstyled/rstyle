//! Bounded, validated baseline artifacts share one create-new publication boundary.
use serde::{Serialize, de::DeserializeOwned};
use std::{fs::File, io::Read, path::Path};

pub(super) const MAX_BYTES: usize = 4 * 1024 * 1024;

pub(in crate::clones) trait Artifact: Serialize + DeserializeOwned {
    const LABEL: &'static str;

    fn validate(&self) -> Result<(), String>;
}

pub(super) fn load<T: Artifact>(path: &Path) -> Result<T, String> {
    let bytes = read_bounded(path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes = Vec::new();
    File::open(path).and_then(|file| file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))?;
    Ok(bytes)
}

pub(super) fn parse<T: Artifact>(bytes: &[u8]) -> Result<T, String> {
    bounded::<T>(bytes)?;
    let artifact: T = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    artifact.validate()?;
    Ok(artifact)
}

pub(super) fn publish<T: Artifact>(path: &Path, artifact: &T) -> Result<(), String> {
    let bytes = encode(artifact)?;

    super::publication::write(path, &bytes, super::publication::Mode::CreateNew)
}

pub(super) fn encode<T: Artifact>(artifact: &T) -> Result<Vec<u8>, String> {
    artifact.validate()?;
    let bytes = serde_json::to_vec_pretty(artifact).map_err(|error| error.to_string())?;
    bounded::<T>(&bytes)?;
    Ok(bytes)
}

fn bounded<T: Artifact>(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_BYTES {
        return Err(format!("{} exceeds the 4 MiB byte limit", T::LABEL));
    }
    Ok(())
}
