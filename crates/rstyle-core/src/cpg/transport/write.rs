use super::{Encoding, Observed, SealedDocument, validate_object};
use serde::Serialize;
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

struct Counted<W> {
    inner: W,
    observed: Observed,
}
impl<W: Write> Write for Counted<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let len = self.inner.write(bytes)?;
        self.observed.update(&bytes[..len])?;
        Ok(len)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Streams trusted structured JSON without constructing a serialized document buffer.
pub fn write_json(
    path: &Path,
    encoding: Encoding,
    value: &impl Serialize,
) -> Result<SealedDocument, String> {
    write(path, encoding, false, |writer| {
        serde_json::to_writer(writer, value).map_err(io::Error::other)
    })
}

/// Streams a frontend's JSON bytes and validates their syntax before publication.
pub fn publish(
    path: &Path,
    encoding: Encoding,
    emit: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> Result<SealedDocument, String> {
    write(path, encoding, true, emit)
}

fn write(
    path: &Path,
    encoding: Encoding,
    validate: bool,
    emit: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> Result<SealedDocument, String> {
    let parent = path.parent().ok_or("Document has no output directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    let (stored, decoded) =
        encode(temporary.as_file_mut(), encoding, emit).map_err(|e| e.to_string())?;
    if validate {
        validate_object(temporary.path(), encoding)?;
    }
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Document filename is not UTF-8")?
        .to_owned();
    let seal = SealedDocument {
        file,
        bytes: stored.bytes,
        checksum: stored.checksum(),
        encoding,
        decoded_bytes: Some(decoded.bytes),
        decoded_checksum: Some(decoded.checksum()),
    };
    temporary
        .persist_noclobber(path)
        .map_err(|error| format!("Cannot publish {}: {error}", path.display()))?;
    Ok(seal)
}

fn encode(
    file: impl Write,
    encoding: Encoding,
    emit: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<(Observed, Observed)> {
    let mut stored = Counted {
        inner: BufWriter::new(file),
        observed: Observed::default(),
    };
    let decoded = match encoding {
        Encoding::Json => emitted(&mut stored, emit)?,
        Encoding::Zstd => {
            let mut encoder = zstd::stream::write::Encoder::new(&mut stored, 3)?;
            encoder.include_checksum(true)?;
            let decoded = emitted(&mut encoder, emit)?;
            encoder.finish()?;
            decoded
        }
    };
    stored.flush()?;
    Ok((stored.observed, decoded))
}

fn emitted(
    writer: impl Write,
    emit: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<Observed> {
    let mut decoded = Counted {
        inner: writer,
        observed: Observed::default(),
    };
    emit(&mut decoded)?;
    decoded.flush()?;
    Ok(decoded.observed)
}
