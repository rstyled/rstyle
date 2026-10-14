//! Exact transport-byte seal, separate from the raw unit evidence identity.
use super::*;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

const SCHEMA: &str = "rstyle.shared-unit.manifest/1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleManifest {
    pub schema: String,
    pub sha256: String,
    pub bytes: u64,
}

impl BundleManifest {
    pub fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        let len = u64::try_from(bytes.len()).map_err(|_| "Oversized shared unit transport")?;
        if self.schema != SCHEMA
            || self.bytes != len
            || self.sha256 != format!("sha256:{:x}", Sha256::digest(bytes))
        {
            return Err("Shared unit transport bytes disagree with driver manifest".into());
        }
        Ok(())
    }
}

struct Recorder<W> {
    writer: W,
    hash: Sha256,
    bytes: u64,
}

impl<W: Write> Write for Recorder<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.writer.write(bytes)?;
        self.hash.update(&bytes[..count]);
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(count).map_err(io::Error::other)?)
            .ok_or_else(|| io::Error::other("Oversized shared unit transport"))?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl UnitStore {
    /// Stream and seal the same bytes, including a checked final flush.
    pub fn write_sealed_bundle(&self, writer: impl Write) -> Result<BundleManifest, String> {
        let mut recorder = Recorder {
            writer,
            hash: Sha256::new(),
            bytes: 0,
        };
        self.write_bundle(&mut recorder)?;
        recorder.flush().map_err(|error| error.to_string())?;

        Ok(BundleManifest {
            schema: SCHEMA.into(),
            sha256: format!("sha256:{:x}", recorder.hash.finalize()),
            bytes: recorder.bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ShortWriter(Vec<u8>);

    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let count = bytes.len().min(3);
            self.0.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FlushFailure;

    impl Write for FlushFailure {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("injected flush failure"))
        }
    }

    #[test]
    fn seal_tracks_actual_partial_writes_and_rejects_changed_bytes() {
        let store = UnitStore::capture(Vec::new()).unwrap();
        let mut writer = ShortWriter(Vec::new());
        let manifest = store.write_sealed_bundle(&mut writer).unwrap();
        manifest.verify(&writer.0).unwrap();
        assert_eq!(manifest.bytes, u64::try_from(writer.0.len()).unwrap());
        writer.0.push(b' ');
        assert!(manifest.verify(&writer.0).is_err());
    }

    #[test]
    fn failed_final_flush_cannot_publish_a_valid_seal() {
        let store = UnitStore::capture(Vec::new()).unwrap();
        assert!(
            store
                .write_sealed_bundle(FlushFailure)
                .unwrap_err()
                .contains("flush failure")
        );
    }
}
