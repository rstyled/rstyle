//! Exact transport-byte seal, separate from the raw unit evidence identity.
use super::*;
use std::io::{self, Write};
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

const SCHEMA: &str = "rstyle.shared-unit.manifest/2";

/// Nonadversarial byte-consistency checksum, never evidence authorization.
pub fn transport_checksum(bytes: &[u8]) -> String {
    encode(xxh3_64(bytes))
}

fn encode(value: u64) -> String {
    format!("xxh3-64:{value:016x}")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleManifest {
    pub schema: String,
    pub checksum: String,
    pub bytes: u64,
}

/// Owned transport bytes checked against a publication seal. This establishes
/// byte consistency only; importing still checks selected context and evidence.
/// Private storage prevents mutation after verification.
pub struct VerifiedBundle {
    bytes: Vec<u8>,
}

impl VerifiedBundle {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl BundleManifest {
    /// Verify once, then move the exact checked buffer into immutable ownership.
    pub fn verify_owned(&self, bytes: Vec<u8>) -> Result<VerifiedBundle, String> {
        self.verify(&bytes)?;
        Ok(VerifiedBundle { bytes })
    }

    pub fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        let len = u64::try_from(bytes.len()).map_err(|_| "Oversized shared unit transport")?;
        if self.schema != SCHEMA || self.bytes != len || self.checksum != transport_checksum(bytes)
        {
            return Err("Shared unit transport bytes disagree with driver manifest".into());
        }
        Ok(())
    }
}

struct Recorder<W> {
    writer: W,
    hash: Xxh3,
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
            hash: Xxh3::new(),
            bytes: 0,
        };
        self.write_bundle(&mut recorder)?;
        recorder.flush().map_err(|error| error.to_string())?;

        Ok(BundleManifest {
            schema: SCHEMA.into(),
            checksum: encode(recorder.hash.digest()),
            bytes: recorder.bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_checksum_matches_vectors_and_chunk_boundaries() {
        assert_eq!(transport_checksum(b""), "xxh3-64:2d06800538d394c2");
        assert_eq!(transport_checksum(b"abc"), "xxh3-64:78af5f94892f3950");
        let bytes: Vec<_> = (0..65537).map(|index| (index % 251) as u8).collect();
        for size in [1, 3, 16, 64, 255, 1024, 4096, 65536] {
            let mut hash = Xxh3::new();
            for chunk in bytes.chunks(size) {
                hash.update(chunk);
            }
            assert_eq!(encode(hash.digest()), transport_checksum(&bytes));
        }
    }

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
        let mut changed = writer.0.clone();
        changed[0] ^= 1;
        assert!(manifest.verify(&changed).is_err());
        writer.0.push(b' ');
        assert!(manifest.verify(&writer.0).is_err());
    }

    #[test]
    fn owned_snapshot_retains_checked_bytes_and_import_inspection() {
        let store = UnitStore::capture(Vec::new()).unwrap();
        let mut bytes = Vec::new();
        let manifest = store.write_sealed_bundle(&mut bytes).unwrap();
        let snapshot = manifest.verify_owned(bytes.clone()).unwrap();
        bytes.push(b' ');
        assert!(manifest.verify_owned(bytes).is_err());
        manifest.verify(snapshot.bytes()).unwrap();
        UnitStore::from_verified_current_run(&snapshot, |_| Ok(())).unwrap();
        let error =
            UnitStore::from_verified_current_run(&snapshot, |_| Err("wrong selected unit".into()));
        assert!(matches!(error, Err(error) if error == "wrong selected unit"));
        let mut wrong = manifest.clone();
        wrong.schema.push('x');
        assert!(wrong.verify_owned(snapshot.bytes().to_vec()).is_err());
        wrong = manifest.clone();
        wrong.bytes += 1;
        assert!(wrong.verify_owned(snapshot.bytes().to_vec()).is_err());
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
