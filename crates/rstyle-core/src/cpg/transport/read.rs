use super::{Encoding, Observed, SealedDocument};
use serde::{
    Deserialize,
    de::{DeserializeOwned, IgnoredAny, MapAccess, Visitor},
};
use std::{
    fmt,
    fs::File,
    io::{self, BufReader, Read, Seek},
    path::Path,
};

/// IgnoredAny skips string decoding, so syntax-only ingestion also validates UTF-8
/// across read boundaries without retaining long source-content strings.
struct Utf8<R> {
    inner: R,
    pending: [u8; 4],
    len: usize,
}

impl<R> Utf8<R> {
    fn check(&mut self, bytes: &[u8]) -> io::Result<()> {
        let bytes = self.complete_pending(bytes)?;

        if let Err(error) = std::str::from_utf8(bytes) {
            if error.error_len().is_some() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, error));
            }
            let suffix = &bytes[error.valid_up_to()..];
            self.pending[..suffix.len()].copy_from_slice(suffix);
            self.len = suffix.len();
        }
        Ok(())
    }

    fn complete_pending<'a>(&mut self, bytes: &'a [u8]) -> io::Result<&'a [u8]> {
        if self.len > 0 {
            let width = match self.pending[0] {
                0..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => 4,
            };
            let count = (width - self.len).min(bytes.len());
            self.pending[self.len..self.len + count].copy_from_slice(&bytes[..count]);
            self.len += count;
            if self.len < width {
                return Ok(&[]);
            }
            std::str::from_utf8(&self.pending[..width])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            self.len = 0;
            return Ok(&bytes[count..]);
        }
        Ok(bytes)
    }

    fn complete_read(&self, len: usize) -> io::Result<usize> {
        if len == 0 && self.len > 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Incomplete UTF-8 in CPG JSON",
            ));
        }
        Ok(len)
    }
}

impl<R: Read> Read for Utf8<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }

        let len = self.inner.read(bytes)?;
        self.check(&bytes[..len])?;
        self.complete_read(len)
    }
}

struct Counted<R> {
    inner: R,
    observed: Observed,
    limit: Option<u64>,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(bytes)?;
        self.observed.update(&bytes[..len])?;
        if self.limit.is_some_and(|limit| self.observed.bytes > limit) {
            return Err(io::Error::other("CPG transport exceeds its declared size"));
        }
        Ok(len)
    }
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    read(path, Encoding::from_path(path), None)
}

pub fn read_sealed<T: DeserializeOwned>(
    directory: &Path,
    seal: &SealedDocument,
) -> Result<T, String> {
    read(&directory.join(&seal.file), seal.encoding, Some(seal))
}

fn read<T: DeserializeOwned>(
    path: &Path,
    encoding: Encoding,
    seal: Option<&SealedDocument>,
) -> Result<T, String> {
    Source {
        path,
        encoding,
        seal,
    }
    .read()
}

struct Source<'a> {
    path: &'a Path,
    encoding: Encoding,
    seal: Option<&'a SealedDocument>,
}

fn open(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|error| {
        format!(
            "missing captured artifact {}: {error}",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    })
}

impl Source<'_> {
    fn read<T: DeserializeOwned>(&self) -> Result<T, String> {
        let mut stored = self.stored()?;
        let value = self.decoded(&mut stored).map_err(|e| self.context(e))?;
        self.verify(&stored.observed)?;
        Ok(value)
    }

    fn stored(&self) -> Result<Counted<File>, String> {
        let mut stored = Counted::new(open(self.path)?, self.seal.map(|s| s.bytes));
        if let Some(seal) = self.seal {
            stored.verify_and_rewind(seal)?;
        }
        Ok(stored)
    }

    fn decoded<T: DeserializeOwned>(&self, stored: &mut Counted<File>) -> Result<T, String> {
        match self.encoding {
            Encoding::Json => parsed(BufReader::new(stored), self.seal),
            Encoding::Zstd => parsed(decoder(stored)?, self.seal),
        }
    }

    fn verify(&self, observed: &Observed) -> Result<(), String> {
        self.seal
            .map_or(Ok(()), |seal| seal.verify_stored(observed))
    }

    fn context(&self, error: String) -> String {
        format!("{}: {error}", self.path.display())
    }
}

impl<R> Counted<R> {
    fn new(inner: R, limit: Option<u64>) -> Self {
        Self {
            inner,
            observed: Observed::default(),
            limit,
        }
    }
}

impl Counted<File> {
    fn verify_and_rewind(&mut self, seal: &SealedDocument) -> Result<(), String> {
        io::copy(self, &mut io::sink()).map_err(|error| error.to_string())?;
        seal.verify_stored(&self.observed)?;

        self.inner.rewind().map_err(|error| error.to_string())?;
        self.observed = Observed::default();
        Ok(())
    }
}

fn decoder<R: Read>(
    reader: R,
) -> Result<zstd::stream::read::Decoder<'static, BufReader<R>>, String> {
    let mut decoder =
        zstd::stream::read::Decoder::new(reader).map_err(|error| error.to_string())?;
    decoder
        .window_log_max(27)
        .map_err(|error| error.to_string())?;
    Ok(decoder)
}

fn parsed<T: DeserializeOwned>(
    reader: impl Read,
    seal: Option<&SealedDocument>,
) -> Result<T, String> {
    let mut decoded = Counted {
        inner: reader,
        observed: Observed::default(),
        limit: seal.and_then(|s| s.decoded_bytes),
    };

    let utf8 = Utf8 {
        inner: &mut decoded,
        pending: [0; 4],
        len: 0,
    };
    let mut json = serde_json::Deserializer::from_reader(BufReader::new(utf8));
    let value = T::deserialize(&mut json).map_err(|error| error.to_string())?;
    json.end().map_err(|error| error.to_string())?;
    if let Some(seal) = seal {
        seal.verify_decoded(&decoded.observed)?;
    }
    Ok(value)
}

/// Syntax-only object validation without retaining a document tree.
pub fn validate_object(path: &Path, encoding: Encoding) -> Result<(), String> {
    let _: Object = read(path, encoding, None)?;
    Ok(())
}

struct Object;
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a CPG JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Object, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Object)
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_validation_crosses_buffer_boundaries_without_a_string_buffer() {
        use std::io::Read;
        for text in ["λ🦀", "hello λ world", "🦀"].map(str::as_bytes) {
            let mut reader = Utf8 {
                inner: text,
                pending: [0; 4],
                len: 0,
            };
            let mut byte = [0];
            let mut actual = Vec::new();
            while reader.read(&mut byte).unwrap() != 0 {
                actual.extend_from_slice(&byte);
            }
            assert_eq!(actual, text);
        }
        let mut reader = Utf8 {
            inner: b"\xf0\x9f".as_slice(),
            pending: [0; 4],
            len: 0,
        };
        assert!(reader.read_to_end(&mut Vec::new()).is_err());
    }
}
