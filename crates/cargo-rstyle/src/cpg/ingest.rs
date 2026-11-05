//! RFC 7464 JSON text sequence: object records, then a terminal null record.
use rstyle_core::cpg::transport::{self, DocumentManifest, Encoding, SealedDocument};
use std::{
    io::{self, BufRead, Read},
    path::Path,
};

const RS: u8 = 0x1e;
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
        Err(error) => Err(format!(
            "Cannot inspect CPG output {}: {error}",
            out.display()
        )),
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
    fn receive(mut self) -> Result<Seals, String> {
        let mut seals = Vec::new();
        while next(self.input)? {
            seals.push(self.document(seals.len())?);
        }
        Ok(seals)
    }

    fn document(&mut self, position: usize) -> Result<SealedDocument, String> {
        document(self.input, self.directory, self.encoding, position)
    }
}

fn document(
    input: &mut impl BufRead,
    directory: &Path,
    encoding: Encoding,
    position: usize,
) -> Result<transport::SealedDocument, String> {
    transport::publish(
        &directory.join(format!("document-{position:06}{}", encoding.suffix())),
        encoding,
        |writer| Record { input }.copy_to(writer),
    )
}

fn next(input: &mut impl BufRead) -> Result<bool, String> {
    match start_record(input)? {
        true => Ok(true),
        false => end(input).map(|()| false),
    }
}

fn start_record(input: &mut impl BufRead) -> Result<bool, String> {
    separator(input)?;
    let first = input
        .fill_buf()
        .map_err(|e| e.to_string())?
        .first()
        .copied();
    is_document(first)
}

fn is_document(first: Option<u8>) -> Result<bool, String> {
    match first {
        Some(b'n') => Ok(false),
        Some(b'{') => Ok(true),
        _ => {
            Err("Each capture record must start with a JSON object; finish with RS null LF".into())
        }
    }
}

fn separator(input: &mut impl Read) -> Result<(), String> {
    let mut byte = [0];
    input
        .read_exact(&mut byte)
        .map_err(|_| "Capture stream ended before its terminal null record")?;
    expect_separator(byte[0])
}

fn expect_separator(byte: u8) -> Result<(), String> {
    if byte != RS {
        return Err("Expected a JSON text sequence record separator (0x1e)".into());
    }
    Ok(())
}

fn end(input: &mut impl Read) -> Result<(), String> {
    let mut bytes = Vec::new();
    input
        .take(64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    terminal(&bytes)
}

fn terminal(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() >= 64 || std::str::from_utf8(bytes).map(str::trim).ok() != Some("null") {
        return Err("The terminal null record must be followed only by whitespace and EOF".into());
    }
    Ok(())
}

/// Stops before the next RS without retaining a JSON document in memory.
struct Record<'a, R> {
    input: &'a mut R,
}
impl<R: BufRead> Record<'_, R> {
    fn copy_to(&mut self, writer: &mut dyn io::Write) -> io::Result<()> {
        io::copy(self, writer)?;
        Ok(())
    }
}

impl<R: BufRead> Read for Record<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let bytes = self.input.fill_buf()?;

        let end = record_length(bytes, out.len());
        out[..end].copy_from_slice(&bytes[..end]);
        self.input.consume(end);
        Ok(end)
    }
}

fn record_length(bytes: &[u8], capacity: usize) -> usize {
    bytes
        .iter()
        .position(|b| *b == RS)
        .unwrap_or(bytes.len())
        .min(capacity)
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
