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
impl<W> Counted<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            observed: Observed::default(),
        }
    }
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
    Emission(emit)
        .stage(path, encoding)?
        .publish(path, validate)
}

struct Staged {
    temporary: tempfile::NamedTempFile,
    seal: SealedDocument,
}

impl Staged {
    fn publish(self, path: &Path, validate: bool) -> Result<SealedDocument, String> {
        if validate {
            validate_object(self.temporary.path(), self.seal.encoding)?;
        }
        persist(self.temporary, path)?;
        Ok(self.seal)
    }
}

/// A serializer callback is consumed once, whether writing plain or compressed JSON.
struct Emission<F>(F);

impl<F: FnOnce(&mut dyn Write) -> io::Result<()>> Emission<F> {
    fn stage(self, path: &Path, encoding: Encoding) -> Result<Staged, String> {
        self.staged(path, encoding, temporary(path)?)
    }

    fn staged(
        self,
        path: &Path,
        encoding: Encoding,
        mut temporary: tempfile::NamedTempFile,
    ) -> Result<Staged, String> {
        let (stored, decoded) = self
            .encode(temporary.as_file_mut(), encoding)
            .map_err(|e| e.to_string())?;
        Ok(Staged {
            temporary,
            seal: sealed(path, encoding, stored, decoded)?,
        })
    }

    fn encode(self, file: impl Write, encoding: Encoding) -> io::Result<(Observed, Observed)> {
        let mut stored = Counted::new(BufWriter::new(file));
        let decoded = self.decoded(&mut stored, encoding)?;
        stored.finished(decoded)
    }

    fn decoded(self, writer: &mut impl Write, encoding: Encoding) -> io::Result<Observed> {
        match encoding {
            Encoding::Json => emitted(writer, self.0),
            Encoding::Zstd => self.compressed(writer),
        }
    }

    fn compressed(self, writer: impl Write) -> io::Result<Observed> {
        let mut encoder = zstd::stream::write::Encoder::new(writer, 3)?;
        encoder.include_checksum(true)?;

        let decoded = emitted(&mut encoder, self.0)?;
        encoder.finish()?;
        Ok(decoded)
    }
}

impl<W: Write> Counted<W> {
    fn finished(mut self, decoded: Observed) -> io::Result<(Observed, Observed)> {
        self.flush()?;
        Ok((self.observed, decoded))
    }
}

fn temporary(path: &Path) -> Result<tempfile::NamedTempFile, String> {
    let parent = path.parent().ok_or("Document has no output directory")?;
    tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())
}

fn sealed(
    path: &Path,
    encoding: Encoding,
    stored: Observed,
    decoded: Observed,
) -> Result<SealedDocument, String> {
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Document filename is not UTF-8")?
        .to_owned();
    Ok(SealedDocument {
        file,
        bytes: stored.bytes,
        checksum: stored.checksum(),
        encoding,
        decoded_bytes: Some(decoded.bytes),
        decoded_checksum: Some(decoded.checksum()),
    })
}

fn persist(temporary: tempfile::NamedTempFile, path: &Path) -> Result<(), String> {
    temporary
        .persist_noclobber(path)
        .map_err(|error| format!("Cannot publish {}: {error}", path.display()))?;
    Ok(())
}

fn emitted(
    writer: impl Write,
    emit: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<Observed> {
    let mut decoded = Counted::new(writer);
    buffered(&mut decoded, emit)?;
    Ok(decoded.observed)
}

fn buffered(
    writer: impl Write,
    emit: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    // JSON serializers write punctuation and short keys separately. Coalesce
    // those calls before hashing and entering the compressor, without retaining
    // the document's serialized representation.
    let mut buffered = BufWriter::with_capacity(64 * 1024, writer);
    emit(&mut buffered)?;
    buffered.flush()
}
