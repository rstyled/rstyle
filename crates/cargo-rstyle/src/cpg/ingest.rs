//! RFC 7464 JSON text sequence: object records, then a terminal null record.
use rstyle_core::cpg::transport::{self, DocumentManifest, Encoding};
use std::{
    io::{self, BufRead, Read},
    path::Path,
};

const RS: u8 = 0x1e;

/// One process receives the capture. EOF before the terminal null is failure.
pub(super) fn run(
    input: &mut impl BufRead,
    out: &Path,
    encoding: Encoding,
) -> Result<(String, u8), String> {
    if std::fs::symlink_metadata(out).is_ok() {
        return Err("CPG output must be a new directory".into());
    }
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stage = tempfile::Builder::new()
        .prefix(".cpg-ingest-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let seals = receive(input, stage.path(), encoding)?;
    if seals.is_empty() {
        return Err("A capture must contain at least one document".into());
    }
    let count = seals.len();
    transport::write_json(
        &stage.path().join(transport::DOCUMENTS_MANIFEST),
        Encoding::Json,
        &DocumentManifest::of(seals),
    )?;
    if std::fs::symlink_metadata(out).is_ok() {
        return Err("CPG output must be a new directory".into());
    }
    std::fs::rename(stage.path(), out).map_err(|e| format!("Cannot publish capture: {e}"))?;
    Ok((
        format!("{count} compat.cpg documents written to {}", out.display()),
        0,
    ))
}

fn receive(
    input: &mut impl BufRead,
    directory: &Path,
    encoding: Encoding,
) -> Result<Vec<transport::SealedDocument>, String> {
    separator(input)?;
    let mut seals = Vec::new();
    loop {
        match input.fill_buf().map_err(|e| e.to_string())?.first() {
            Some(b'n') => {
                end(input)?;
                break;
            }
            Some(b'{') => {}
            _ => {
                return Err(
                    "Each capture record must start with a JSON object; finish with RS null LF"
                        .into(),
                );
            }
        }
        let path = directory.join(format!("document-{:06}{}", seals.len(), encoding.suffix()));
        let seal = transport::publish(&path, encoding, |writer| {
            io::copy(&mut Record { input: &mut *input }, writer)?;
            Ok(())
        })?;
        seals.push(seal);
        separator(input)?;
    }
    Ok(seals)
}

fn separator(input: &mut impl Read) -> Result<(), String> {
    let mut byte = [0];
    input
        .read_exact(&mut byte)
        .map_err(|_| "Capture stream ended before its terminal null record")?;
    if byte[0] != RS {
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
    if bytes.len() >= 64 || std::str::from_utf8(&bytes).map(str::trim).ok() != Some("null") {
        return Err("The terminal null record must be followed only by whitespace and EOF".into());
    }
    Ok(())
}

/// Stops before the next RS without retaining a JSON document in memory.
struct Record<'a, R> {
    input: &'a mut R,
}
impl<R: BufRead> Read for Record<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let bytes = self.input.fill_buf()?;
        let end = bytes
            .iter()
            .position(|b| *b == RS)
            .unwrap_or(bytes.len())
            .min(out.len());
        out[..end].copy_from_slice(&bytes[..end]);
        self.input.consume(end);
        Ok(end)
    }
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
