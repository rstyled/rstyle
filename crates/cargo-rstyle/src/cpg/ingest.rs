//! RFC 7464 JSON text sequence: object records, then a terminal null record.
use rstyle_core::cpg::transport::{
    self, DocumentManifest, Encoding, SealedDocument, SequenceReader,
};
use std::{
    io::{self, BufRead, Read, Write},
    path::Path,
};

type Seals = Vec<SealedDocument>;

/// One process receives the capture. EOF before the terminal null is failure.
pub(super) fn run(
    input: &mut impl BufRead,
    out: &Path,
    encoding: Encoding,
) -> Result<(String, u8), String> {
    StagedCapture::receive(input, out, encoding)?.publish(out)
}

struct StagedCapture {
    directory: tempfile::TempDir,
    documents: usize,
}

impl StagedCapture {
    fn receive(input: &mut impl BufRead, out: &Path, encoding: Encoding) -> Result<Self, String> {
        let directory = stage(out)?;
        let manifest = manifest(receive(input, directory.path(), encoding)?)?;
        Self::seal(directory, manifest)
    }

    fn seal(directory: tempfile::TempDir, manifest: DocumentManifest) -> Result<Self, String> {
        transport::write_json(
            &directory.path().join(transport::DOCUMENTS_MANIFEST),
            Encoding::Json,
            &manifest,
        )?;
        Ok(Self {
            directory,
            documents: manifest.documents.len(),
        })
    }

    fn publish(self, out: &Path) -> Result<(String, u8), String> {
        require_new(out)?;
        std::fs::rename(self.directory.path(), out)
            .map_err(|e| format!("Cannot publish capture: {e}"))?;
        Ok(completed(out, self.documents))
    }
}

fn completed(out: &Path, documents: usize) -> (String, u8) {
    (
        format!(
            "{documents} compat.cpg documents written to {}",
            out.display()
        ),
        0,
    )
}

fn require_new(out: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(out) {
        Ok(_) => Err("CPG output must be a new directory".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", out.display())),
    }
}

fn stage(out: &Path) -> Result<tempfile::TempDir, String> {
    require_new(out)?;
    tempfile::Builder::new()
        .prefix(".cpg-ingest-")
        .tempdir_in(parent(out))
        .map_err(|e| e.to_string())
}

fn parent(out: &Path) -> &Path {
    out.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn manifest(seals: Seals) -> Result<DocumentManifest, String> {
    match seals.len() {
        0 => Err("A capture must contain at least one document".into()),
        _ => Ok(DocumentManifest::of(seals)),
    }
}

fn receive(
    input: &mut impl BufRead,
    directory: &Path,
    encoding: Encoding,
) -> Result<Seals, String> {
    Receiver {
        input,
        directory,
        encoding,
    }
    .receive()
}

struct Receiver<'a, R> {
    input: &'a mut R,
    directory: &'a Path,
    encoding: Encoding,
}

impl<R: BufRead> Receiver<'_, R> {
    fn receive(self) -> Result<Seals, String> {
        let mut reader = SequenceReader::new(self.input);
        let mut seals = Vec::new();

        while let Some(seal) = reader.next_with(|record| {
            stored_document(record, self.directory, self.encoding, seals.len())
        })? {
            seals.push(seal);
        }
        reader.complete()?;
        Ok(seals)
    }
}

fn stored_document(
    input: &mut dyn Read,
    directory: &Path,
    encoding: Encoding,
    position: usize,
) -> Result<SealedDocument, String> {
    let path = directory.join(format!("document-{position:06}{}", encoding.suffix()));
    transport::publish(&path, encoding, |writer| copy_record(input, writer))
}

fn copy_record(input: &mut dyn Read, writer: &mut dyn Write) -> io::Result<()> {
    io::copy(input, writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_or_invalid_streams_publish_nothing() {
        for mut bytes in [
            b"\x1e{}".as_slice(),
            b"\x1e{\x1enull\n",
            b"\x1enull\n",
            b"\x1e{}\x1enull\n\x1e{}",
            b"\x1e{}\x1e[]",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let out = dir.path().join("capture");
            assert!(run(&mut bytes, &out, Encoding::Zstd).is_err());
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn records_keep_their_original_bytes_and_order() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capture");
        let input = b"\x1e{\n\"first\":true\n}\n\x1e{\"second\":2}\n\x1enull\n";
        run(&mut input.as_slice(), &out, Encoding::Zstd).unwrap();
        let manifest = DocumentManifest::read(
            &std::fs::read(out.join(transport::DOCUMENTS_MANIFEST)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest.documents.len(), 2);
        let first: serde_json::Value =
            transport::read_sealed(&out, &manifest.documents[0]).unwrap();
        assert_eq!(first, serde_json::json!({"first":true}));
        use sha2::Digest;
        let original = b"{\n\"first\":true\n}\n";
        assert_eq!(
            manifest.documents[0].decoded_bytes,
            Some(original.len() as u64)
        );
        assert_eq!(
            manifest.documents[0].decoded_checksum.as_deref(),
            Some(format!("sha256:{:x}", sha2::Sha256::digest(original)).as_str())
        );
        assert!(run(&mut input.as_slice(), &out, Encoding::Zstd).is_err());
    }
}
